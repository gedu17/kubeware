use std::sync::{Arc};
use crate::services::Services;
use hyper::{Client, Request, Body, Response};
use hyper::client::HttpConnector;
use crate::config::Config;
use crate::request_container::RequestContainer;
use std::time::{Instant};
use hyper::service::Service;
use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};
use crate::kubeware::{ResponseStatus};

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
        let uri = [config.backend.url.as_str(), req.uri().path()].join("");
        let mut container = RequestContainer::from_request(req).await?;
        let services = services.to_owned();

        for client in services.pre_request() {
            let pre_request_timer = Instant::now();
            match client.connection().clone() {
                Some(mut connection) => {
                    let request = tonic::Request::new(container.generate_pre_request()?);

                    match connection.handle_pre_request(request).await {
                        Ok(response) => {
                            let data = response.into_inner();

                            match ResponseStatus::from_i32(data.status) {
                                Some(ResponseStatus::Success) => container.parse_pre_request(&data)?,
                                Some(ResponseStatus::FailureContinue) => (),
                                Some(ResponseStatus::Stop) => {
                                    container.parse_pre_request(&data)?;

                                    return Ok(container.response_from_pre_request()?)
                                },
                                None => ()
                            };

                            ()
                        },
                        Err(err) => {
                            error!("[Pre Request] Failed to get response from {}: {:?}", client.url(), err);

                            return Ok(RequestHandler::service_unavailable_error())
                        }
                    };

                    ()
                },
                None => {
                    warn!("[Pre Request] Endpoint is not resolved and will be ignored. {}", client.url());

                    ()
                }
            };

            info!("[Pre Request] {} took {} ms", client.url(), pre_request_timer.elapsed().as_millis());
        }

        // Backend request
        let backend_timer = Instant::now();

        match http_client.request(container.request_from_pre_request(uri)?).await {
            Ok(data) => {
                container.backend_elapsed(backend_timer.elapsed());
                container.parse_be_response(data).await?;

                ()
            },
            Err(err) => {
                error!("[Upstream] Failed to get response from upstream. {}", err);

                return Ok(RequestHandler::gateway_error())
            }
        }

        for client in services.post_request() {
            let post_request_timer = Instant::now();

            match client.connection().clone() {
                Some(mut connection) => {
                    let request = tonic::Request::new(container.generate_post_request()?);

                    match connection.handle_post_request(request).await {
                        Ok(response) => {
                            let data = response.into_inner();

                            match ResponseStatus::from_i32(data.status) {
                                Some(ResponseStatus::Success) => container.parse_post_request(&data)?,
                                Some(ResponseStatus::FailureContinue) => (),
                                Some(ResponseStatus::Stop) => {
                                    container.parse_post_request(&data)?;

                                    return Ok(container.response_from_post_request()?)
                                },
                                None => ()
                            }

                            ()
                        },
                        Err(err) => {
                            error!("[Post Request] Failed to get response from {}: {:?}", client.url(), err);

                            return Ok(RequestHandler::service_unavailable_error())
                        }
                    }

                    ()
                },
                None => {
                    warn!("[Post Request] Endpoint is not resolved and will be ignored. {}", client.url());

                    ()
                }
            };

            info!("[Post Request] {} took {} ms", client.url(), post_request_timer.elapsed().as_millis());
        }

        // Response to origin
        Ok(container.response_from_response()?)
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