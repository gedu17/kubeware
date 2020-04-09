use std::convert::Into;
use futures::channel::oneshot;
use std::sync::{Arc, Mutex};
use crate::tower_service::Builder;
use crate::config::{HttpVersion, Config};
use crate::services::Services;
use std::env::set_var;
use crate::{RUST_LOG, LOOPBACK, PORT};

use std::str;
use async_trait::async_trait;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};
use std::net::ToSocketAddrs;
use oneshot::Sender;

use tonic::{transport::Server as TonicServer, Request as TonicRequest, Response as TonicResponse, Status};
use crate::kubeware::{RequestRequest, RequestResponse, ResponseRequest, ResponseResponse};
use crate::kubeware::middleware_server::{Middleware, MiddlewareServer};
use std::sync::atomic::{AtomicUsize, Ordering};

type BootstrapResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type RequestFn = Box<dyn Fn(TonicRequest<RequestRequest>) -> TonicResponse<RequestResponse> + Send + 'static + Sync>;
type ResponseFn = Box<dyn Fn(TonicRequest<ResponseRequest>) -> TonicResponse<ResponseResponse> + Send + 'static + Sync>;

// Tests
mod basic_tests;
mod request_tests;

pub struct MiddlewareService
{
    request_fn: RequestFn,
    response_fn: ResponseFn,
    request_counter: Arc<AtomicUsize>,
    response_counter: Arc<AtomicUsize>
}

impl MiddlewareService
{
    fn new (request: RequestFn, response: ResponseFn) -> MiddlewareService {
        MiddlewareService {
            request_fn: request,
            response_fn: response,
            request_counter: Arc::new(AtomicUsize::new(0)),
            response_counter: Arc::new(AtomicUsize::new(0))
        }
    }

    fn request_counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.request_counter)
    }

    fn response_counter(&self) -> Arc<AtomicUsize>  {
        Arc::clone(&self.response_counter)
    }
 }

#[tonic::async_trait]
impl Middleware for MiddlewareService {

    async fn handle_request(
        &self,
        request: TonicRequest<RequestRequest>,
    ) -> Result<TonicResponse<RequestResponse>, Status> {
        let _ = self.request_counter.fetch_add(1, Ordering::Relaxed);
        Ok((self.request_fn)(request))
    }

    async fn handle_response(
        &self,
        request: TonicRequest<ResponseRequest>,
    ) -> Result<TonicResponse<ResponseResponse>, Status> {
        let _ = self.response_counter.fetch_add(1, Ordering::Relaxed);
        Ok((self.response_fn)(request))
    }
}

#[allow(dead_code)]
async fn setup_kubeware (config: &str) -> BootstrapResult<Sender<()>> {
    let config: Config = toml::from_str(config)?;
    let address = [config.ip.clone().unwrap_or(LOOPBACK.to_string()), config.port.clone().unwrap_or(PORT).to_string()]
        .join(":")
        .to_socket_addrs()?
        .next()
        .unwrap();

    set_var(RUST_LOG, "INFO");

    let _ = pretty_env_logger::try_init_timed();

    let mut services = Services::with_config(&config);

    for service in &config.services {
        services.insert(service).await?;
    }

    let http_client = match &config.backend.version {
        Some(version) => match version {
            HttpVersion::HTTP => Client::new(),
            HttpVersion::HTTP2 => Client::builder().http2_only(true).build_http()
        },
        _ => Client::new()
    };

    let (tx, rx) = oneshot::channel::<()>();

    let server = Server::bind(&address).serve(Builder {
        http_client,
        config,
        mutex: Mutex::new(false),
        services: Arc::new(services)
    }).with_graceful_shutdown(async move {
        rx.await.ok();
    });

    tokio::task::spawn(async move {
        if let Err(e) = server.await {
            error!("server error: {}", e);
        }
    });

    Ok(tx)
}

#[allow(dead_code)]
async fn setup_backend<F> (closure: F) -> BootstrapResult<(Sender<()>, Arc<AtomicUsize>)>
    where F: Fn(Request<Body>) -> Response<Body> + Send + 'static + Clone + Sync {

    let address = ([127, 0, 0, 1], 17001).into();
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = Arc::clone(&counter);
    let make_service = make_service_fn(move |_| {
        let cloned_closure = closure.clone();
        let cloned_counter = counter.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                let _ = cloned_counter.fetch_add(1, Ordering::Relaxed);
                let result = (cloned_closure)(req);
                async move {
                    Ok::<Response<Body>, hyper::Error>(result)
                }
            }))
        }
    });

    let (backend_tx, backend_rx) = oneshot::channel::<()>();
    let server = Server::bind(&address)
        .serve(make_service)
        .with_graceful_shutdown(async move {
            backend_rx.await.ok();
        });

    tokio::task::spawn(async move {
        if let Err(e) = server.await {
            error!("server error: {}", e);
        }
    });

    Ok((backend_tx, cloned_counter))
}

async fn setup_backend2<F> (obj: F) -> BootstrapResult<(Sender<()>, Arc<AtomicUsize>)>
    where F: BackendResponse + Send + 'static + Clone + Sync {

    let address = ([127, 0, 0, 1], 17001).into();
    let counter = Arc::new(AtomicUsize::new(0));
    let cloned_counter = Arc::clone(&counter);

    let make_service = make_service_fn(move |_| {
        let cloned = obj.clone();
        let cloned_counter = counter.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                let _ = cloned_counter.fetch_add(1, Ordering::Relaxed);
                // let result = cloned.handle(req);
                let mut cloned = cloned.clone();
                async move {
                    Ok::<Response<Body>, hyper::Error>(cloned.handle(req).await)
                }
            }))
        }
    });

    let (backend_tx, backend_rx) = oneshot::channel::<()>();
    let server = Server::bind(&address)
        .serve(make_service)
        .with_graceful_shutdown(async move {
            backend_rx.await.ok();
        });

    tokio::task::spawn(async move {
        if let Err(e) = server.await {
            error!("server error: {}", e);
        }
    });

    Ok((backend_tx, cloned_counter))
}

#[allow(dead_code)]
async fn setup_middleware (request: RequestFn, response: ResponseFn) -> BootstrapResult<(Sender<()>, Arc<AtomicUsize>, Arc<AtomicUsize>)> {

    let service = MiddlewareService::new(request, response);
    let request_counter = service.request_counter();
    let response_counter = service.response_counter();

    let (middleware_tx, middleware_rx) = oneshot::channel::<()>();

    let middleware = TonicServer::builder()
        .add_service(MiddlewareServer::new(service))
        .serve_with_shutdown("127.0.0.1:17002".parse().unwrap(), async move {
            middleware_rx.await.ok();
        });

    tokio::task::spawn(async move {
        if let Err(e) = middleware.await {
            error!("server error: {}", e);
        }
    });

    Ok((middleware_tx, request_counter, response_counter))
}

#[async_trait]
trait BackendResponse {
    async fn handle(&mut self, request: Request<Body>) -> Response<Body>;
}