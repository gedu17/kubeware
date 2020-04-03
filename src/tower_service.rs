use std::sync::{Arc, Mutex};
use hyper::{Client};
use hyper::client::HttpConnector;
use hyper::service::Service;
use std::task::{Context, Poll};
use futures::future;

use crate::services::Services;
use crate::request_handler::RequestHandler;
use crate::config::Config;

pub struct Builder
{
    pub http_client: Client<HttpConnector>,
    pub services: Arc<Services>,
    pub config: Config,
    pub mutex: Mutex<bool>
}

impl<T> Service<T> for Builder {
    type Response = RequestHandler;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let services = Arc::clone(&self.services);
        // TODO: test out latency / mutex on higher load !
        match self.mutex.lock() {
            Ok(_) => {
                if services.all().iter().any(|x| x.connection().is_none()) {
                    debug!("Trying to reconnect to unreachable hosts...");

                    // Is there another way to do this ? :(
                    match futures::executor::block_on(services.ensure_connected()) {
                        Ok(val) => self.services = Arc::new(val),
                        Err(err) => {
                            error!("Failed to ensure are services are resolved. {}", err)
                        }
                    }
                }
            },
            Err(val) => {
                warn!("Failed to acquire mutex. {}", val.to_string())
            }
        }

        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(RequestHandler {
            services: self.services.clone(),
            http_client: self.http_client.clone(),
            config: self.config.clone()
        })
    }
}