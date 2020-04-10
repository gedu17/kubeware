use std::sync::{Arc};
use crate::middlewares::Middlewares;
use hyper::{Client, Request, Body, Response};
use hyper::client::HttpConnector;
use crate::config::{Config};
use std::time::{Instant, Duration};
use hyper::service::Service;
use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};
use crate::kubeware::{ResponseStatus};
use crate::container_handler::ContainerHandler;
use crate::request_container::ContainerState::{MiddlewareResponse, Response as BackendResponse};
use tonic::metadata::{MetadataValue};
use crate::{DEFAULT_TIMEOUT_MILLIS, KUBEWARE_TIME_HEADER};
use hyper::header::HeaderValue;

type HandlerResult<T> = std::result::Result<T, GenericError>;
type GenericError = Box<dyn std::error::Error + Send + Sync>;
const GRPC_TIMEOUT_HEADER: &str = "grpc-timeout";

pub struct RequestHandler
{
    pub middlewares: Arc<Middlewares>,
    pub http_client: Client<HttpConnector>,
    pub config: Config
}

impl RequestHandler {

    fn generic_error() -> Response<Body> {
        let response = Response::builder();
        let body = Body::from(Vec::from(&b"Internal server error"[..]));

        response.status(500).body(body).unwrap()
    }

    fn gateway_error(timer: Instant) -> HandlerResult<Response<Body>> {
        let response = Response::builder()
            .header(KUBEWARE_TIME_HEADER, HeaderValue::from_str(&timer.elapsed().as_millis().to_string())?);
        let body = Body::from(Vec::from(&b"Bad Gateway"[..]));

        Ok(response.status(502).body(body).unwrap())
    }

    fn service_unavailable_error(timer: Instant) -> HandlerResult<Response<Body>> {
        let response = Response::builder()
            .header(KUBEWARE_TIME_HEADER, HeaderValue::from_str(&timer.elapsed().as_millis().to_string())?);
        let body = Body::from(Vec::from(&b"Service Unavailable"[..]));

        Ok(response.status(503).body(body).unwrap())
    }

    fn gateway_timeout(timer: Instant) -> HandlerResult<Response<Body>> {
        let response = Response::builder()
            .header(KUBEWARE_TIME_HEADER, HeaderValue::from_str(&timer.elapsed().as_millis().to_string())?);
        let body = Body::from(Vec::from(&b"Gateway Timeout"[..]));

        Ok(response.status(504).body(body).unwrap())
    }

    async fn handle(req: Request<Body>, middlewares: Arc<Middlewares>, config: Config, http_client: Client<HttpConnector>) -> Result<Response<Body>, GenericError> {
        let url = [config.backend.url.as_str(), req.uri().path()].join("");
        let mut container = ContainerHandler::new(req, url).await?;
        let middlewares = middlewares.to_owned();
        let backend_timeout = Duration::from_millis(config.backend.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MILLIS) as u64);

        for client in middlewares.request() {
            let timer = Instant::now();

            match client.connection().clone() {
                Some(mut connection) => {
                    let timeout = [client.timeout().as_millis().to_string(), "m".to_string()].join("");
                    let mut request = tonic::Request::new(container.into_middleware_request()?);
                    let metadata = request.metadata_mut();
                    metadata.insert(GRPC_TIMEOUT_HEADER, MetadataValue::from_str(timeout.as_str())?);

                    match tokio::time::timeout(client.timeout(), connection.handle_request(request)).await {
                        Ok(val) => {
                            match val {
                                Ok(response) => {
                                    let data = response.into_inner();

                                    match ResponseStatus::from_i32(data.status) {
                                        Some(ResponseStatus::Success) => container.handle_middleware_request(&data, false)?,
                                        Some(ResponseStatus::Continue) => (),
                                        Some(ResponseStatus::Stop) => {
                                            container.handle_middleware_request(&data, true)?;

                                            return Ok(container.into_response()?)
                                        },
                                        None => ()
                                    };
                                },
                                Err(err) => {
                                    error!("[Middleware Request] Failed to get response from {}: {:?}", client.url(), err);

                                    return Ok(RequestHandler::service_unavailable_error(container.timer())?)
                                }
                            }
                        },
                        Err(_err) => {
                            error!("[Middleware Request] Timed out {}: elapsed {} ms.", client.url(), client.timeout().as_millis());

                            return Ok(RequestHandler::service_unavailable_error(container.timer())?)
                        }
                    }
                },
                None => {
                    error!("[Middleware Request] Endpoint is not resolved. {}", client.url());
                    return Ok(RequestHandler::service_unavailable_error(container.timer())?)
                }
            };

            info!("[Middleware Request] {} took {} ms.", client.url(), timer.elapsed().as_millis());
        }

        container.state_set(BackendResponse);

        let backend_timer = Instant::now();

        match tokio::time::timeout(backend_timeout, http_client.request(container.into_request()?)).await {
            Ok(val) => {
                match val {
                    Ok(data) => {
                        container.backend_elapsed_set(backend_timer.elapsed());
                        container.handle_response(data).await?;
                    },
                    Err(err) => {
                        error!("[Backend] Failed to get response from backend. {}", err);

                        return Ok(RequestHandler::gateway_error(container.timer())?)
                    }
                }
            },
            Err(_err) => {
                error!("[Backend] Timed out after {} ms.", backend_timeout.as_millis());

                return Ok(RequestHandler::gateway_timeout(container.timer())?)
            }
        }

        info!("[Backend Request] took {} ms.", container.backend_elapsed().unwrap_or(Duration::from_millis(0)).as_millis());
        container.state_set(MiddlewareResponse);

        for client in middlewares.response() {
            let timer = Instant::now();

            match client.connection().clone() {
                Some(mut connection) => {
                    let timeout = [client.timeout().as_millis().to_string(), "m".to_string()].join("");
                    let mut request = tonic::Request::new(container.into_middleware_response()?);
                    let metadata = request.metadata_mut();
                    metadata.insert(GRPC_TIMEOUT_HEADER, MetadataValue::from_str(timeout.as_str())?);

                    match tokio::time::timeout(client.timeout(), connection.handle_response(request)).await {
                        Ok(val) => {
                            match val {
                                Ok(response) => {
                                    let data = response.into_inner();

                                    match ResponseStatus::from_i32(data.status) {
                                        Some(ResponseStatus::Success) => container.handle_middleware_response(&data, false)?,
                                        Some(ResponseStatus::Continue) => (),
                                        Some(ResponseStatus::Stop) => {
                                            container.handle_middleware_response(&data, true)?;

                                            return Ok(container.into_response()?)
                                        },
                                        None => ()
                                    }
                                },
                                Err(err) => {
                                    error!("[Middleware Response] Failed to get response from {}: {:?}", client.url(), err);

                                    return Ok(RequestHandler::service_unavailable_error(container.timer())?)
                                }
                            }
                        },
                        Err(_err) => {
                            error!("[Middleware Response] Timed out {}: elapsed {} ms.", client.url(), client.timeout().as_millis());

                            return Ok(RequestHandler::service_unavailable_error(container.timer())?)
                        }
                    }
                },
                None => {
                    error!("[Middleware Response] Endpoint is not resolved. {}", client.url());
                    return Ok(RequestHandler::service_unavailable_error(container.timer())?)
                }
            };

            info!("[Middleware Response] {} took {} ms.", client.url(), timer.elapsed().as_millis());
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
        let middlewares = Arc::clone(&self.middlewares);
        let config = self.config.clone();
        let http_client = self.http_client.clone();

        let executor = async move {
            match RequestHandler::handle(req, middlewares, config, http_client).await {
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