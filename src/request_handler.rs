use std::sync::{Arc};
use crate::services::Services;
use hyper::{Client, Request, Body, Response};
use hyper::client::HttpConnector;
use crate::config::{Config};
use std::time::{Instant};
use hyper::service::Service;
use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};
use crate::kubeware::{ResponseStatus};
use crate::container_handler::ContainerHandler;
use crate::request_container::ContainerState::{MiddlewareResponse, Response as BackendResponse};

type GenericError = Box<dyn std::error::Error + Send + Sync>;

pub struct RequestHandler
{
    pub services: Arc<Services>,
    pub http_client: Client<HttpConnector>,
    pub config: Config
}

impl RequestHandler {

    fn generic_error() -> Response<Body> {
        let response = Response::builder();
        let body = Body::from(Vec::from(&b"Internal server error"[..]));

        response.status(500).body(body).unwrap()
    }

    fn gateway_error() -> Response<Body> {
        let response = Response::builder();
        let body = Body::from(Vec::from(&b"Bad Gateway"[..]));

        response.status(502).body(body).unwrap()
    }

    fn service_unavailable_error() -> Response<Body> {
        let response = Response::builder();
        let body = Body::from(Vec::from(&b"Service Unavailable"[..]));

        response.status(503).body(body).unwrap()
    }

    async fn handle(req: Request<Body>, services: Arc<Services>, config: Config, http_client: Client<HttpConnector>) -> Result<Response<Body>, GenericError> {
        let url = [config.backend.url.as_str(), req.uri().path()].join("");
        let mut container = ContainerHandler::new(req, url).await?;
        let services = services.to_owned();

        for client in services.request() {
            let timer = Instant::now();

            match client.connection().clone() {
                Some(mut connection) => {
                    let request = tonic::Request::new(container.into_middleware_request()?);

                    match connection.handle_request(request).await {
                        Ok(response) => {
                            let data = response.into_inner();

                            match ResponseStatus::from_i32(data.status) {
                                Some(ResponseStatus::Success) => container.handle_middleware_request(&data)?,
                                Some(ResponseStatus::Continue) => (),
                                Some(ResponseStatus::Stop) => {
                                    container.handle_middleware_request(&data)?;

                                    return Ok(container.into_response()?)
                                },
                                None => ()
                            };

                            ()
                        },
                        Err(err) => {
                            error!("[Middleware Request] Failed to get response from {}: {:?}", client.url(), err);

                            return Ok(RequestHandler::service_unavailable_error())
                        }
                    };

                    ()
                },
                None => {
                    warn!("[Middleware Request] Endpoint is not resolved and will be ignored. {}", client.url());

                    ()
                }
            };

            info!("[Middleware Request] {} took {} ms", client.url(), timer.elapsed().as_millis());
        }

        container.state_set(BackendResponse);

        let backend_timer = Instant::now();

        match http_client.request(container.into_request()?).await {
            Ok(data) => {
                container.backend_elapsed_set(backend_timer.elapsed());
                container.handle_response(data).await?;

                ()
            },
            Err(err) => {
                error!("[Backend] Failed to get response from backend. {}", err);

                return Ok(RequestHandler::gateway_error())
            }
        }

        container.state_set(MiddlewareResponse);

        for client in services.response() {
            let timer = Instant::now();

            match client.connection().clone() {
                Some(mut connection) => {
                    let request = tonic::Request::new(container.into_middleware_response()?);

                    match connection.handle_response(request).await {
                        Ok(response) => {
                            let data = response.into_inner();

                            match ResponseStatus::from_i32(data.status) {
                                Some(ResponseStatus::Success) => container.handle_middleware_response(&data)?,
                                Some(ResponseStatus::Continue) => (),
                                Some(ResponseStatus::Stop) => {
                                    container.handle_middleware_response(&data)?;

                                    return Ok(container.into_response()?)
                                },
                                None => ()
                            }

                            ()
                        },
                        Err(err) => {
                            error!("[Middleware Request] Failed to get response from {}: {:?}", client.url(), err);

                            return Ok(RequestHandler::service_unavailable_error())
                        }
                    }

                    ()
                },
                None => {
                    warn!("[Middleware Request] Endpoint is not resolved and will be ignored. {}", client.url());

                    ()
                }
            };

            info!("[Middleware Request] {} took {} ms", client.url(), timer.elapsed().as_millis());
        }

        Ok(container.into_response()?)
    }
}

impl Service<Request<Body>> for RequestHandler {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let services = Arc::clone(&self.services);
        let config = self.config.clone();
        let http_client = self.http_client.clone();

        let executor = async move {
            match RequestHandler::handle(req, services, config, http_client).await {
                Ok(val) => Ok(val),
                Err(err) => {
                    error!("Failed to parse request: {:?}", err);

                    Ok(RequestHandler::generic_error())
                }
            }
        };

        Box::pin(executor)
    }
}