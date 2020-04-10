use std::sync::{Arc, Mutex};
use hyper::{Client};
use hyper::client::HttpConnector;
use hyper::service::Service;
use std::task::{Context, Poll};
use futures::future;

use crate::middlewares::Middlewares;
use crate::request_handler::RequestHandler;
use crate::config::Config;

pub struct Builder
{
    pub http_client: Client<HttpConnector>,
    pub middlewares: Arc<Middlewares>,
    pub config: Config,
    pub mutex: Mutex<bool>
}

impl<T> Service<T> for Builder {
    type Response = RequestHandler;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let middlewares = Arc::clone(&self.middlewares);
        
        match self.mutex.lock() {
            Ok(_) => {
                if middlewares.all().iter().any(|x| x.connection().is_none()) {
                    debug!("Trying to reconnect to unreachable hosts...");

                    // Is there another way to do this ? :(
                    match futures::executor::block_on(middlewares.ensure_connected()) {
                        Ok(val) => self.middlewares = Arc::new(val),
                        Err(err) => {
                            error!("Failed to ensure middlewares are resolved. {}", err)
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
            middlewares: self.middlewares.clone(),
            http_client: self.http_client.clone(),
            config: self.config.clone()
        })
    }
}