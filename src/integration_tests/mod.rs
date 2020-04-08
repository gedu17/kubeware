use std::convert::Into;
use futures::channel::oneshot;
use std::sync::{Arc, Mutex};
use crate::tower_service::Builder;
use crate::config::{HttpVersion, Config};
use crate::services::Services;
use std::env::set_var;
use crate::{RUST_LOG, LOOPBACK, PORT};

use std::str;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};
use std::net::ToSocketAddrs;
use oneshot::Sender;

use tonic::{transport::Server as TonicServer, Request as TonicRequest, Response as TonicResponse, Status};
use crate::kubeware::{RequestRequest, RequestResponse, ResponseRequest, ResponseResponse};
use crate::kubeware::middleware_server::{Middleware, MiddlewareServer};
use log::SetLoggerError;

type BootstrapResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type RequestFn = Box<dyn Fn(TonicRequest<RequestRequest>) -> TonicResponse<RequestResponse> + Send + 'static + Sync>;
type ResponseFn = Box<dyn Fn(TonicRequest<ResponseRequest>) -> TonicResponse<ResponseResponse> + Send + 'static + Sync>;
type BackendFn = Box<dyn Fn(Request<Body>) -> Response<Body> + Send + 'static + Sync>; //Pin<Box<dyn Fn(Request<Body>) -> Result<Response<Body>, hyper::Error> + Send + Sync>>;
type BackendFn2 = Box<dyn FnMut(Request<Body>) -> Result<Response<Body>, hyper::Error>>;

// Tests
mod basic_tests;
mod request_tests;
mod tower_service;

pub struct MiddlewareService
{
    request_fn: RequestFn,
    response_fn: ResponseFn
}

impl MiddlewareService
{
    fn new (request: RequestFn, response: ResponseFn) -> MiddlewareService {
        MiddlewareService {
            request_fn: request,
            response_fn: response
        }
    }
}

#[tonic::async_trait]
impl Middleware for MiddlewareService {

    async fn handle_request(
        &self,
        request: TonicRequest<RequestRequest>,
    ) -> Result<TonicResponse<RequestResponse>, Status> {
        info!("Got a request from {:?}", request.remote_addr());

        Ok((self.request_fn)(request))
    }

    async fn handle_response(
        &self,
        request: TonicRequest<ResponseRequest>,
    ) -> Result<TonicResponse<ResponseResponse>, Status> {
        info!("Got a request from {:?}", request.remote_addr());
        Ok((self.response_fn)(request))
    }
}

async fn setup_kubeware (config: &str) -> BootstrapResult<Sender<()>> {
    let config: Config = toml::from_str(config)?;
    let address = [config.ip.clone().unwrap_or(LOOPBACK.to_string()), config.port.clone().unwrap_or(PORT).to_string()]
        .join(":")
        .to_socket_addrs()?
        .next()
        .unwrap();

    set_var(RUST_LOG, "INFO");

    pretty_env_logger::try_init_timed();

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

// async fn setup_backend (closure: BackendFn) -> BootstrapResult<Sender<()>> {
async fn setup_backend<F> (closure: F) -> BootstrapResult<Sender<()>>
    where F: Fn(Request<Body>) -> Response<Body> + Send + 'static + Copy + Sync {

    let address = ([127, 0, 0, 1], 17001).into();

    let make_service = make_service_fn(move |_| {
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                async move {
                    Ok::<Response<Body>, hyper::Error>(closure(req))
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

    Ok(backend_tx)
}

async fn setup_middleware (request: RequestFn, response: ResponseFn) -> BootstrapResult<Sender<()>> {

    let service = MiddlewareService::new(request, response);

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

    Ok(middleware_tx)
}