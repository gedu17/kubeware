use crate::kubeware::middleware_client::MiddlewareClient;
use crate::config::{MiddlewareConfig, Config};
use crate::middleware::{Middleware, MiddlewareBuilder};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[derive(Clone)]
pub struct Middlewares {
    inner: Vec<Middleware>,
    config: Config
}

impl Middlewares {
    pub fn all(&self) -> &Vec<Middleware> {
        &self.inner
    }

    pub fn request(&self) -> Vec<&Middleware> {
        self.inner.iter().filter(|x| x.request() == true).collect()
    }

    pub fn response(&self) -> Vec<&Middleware> {
        self.inner.iter().filter(|x| x.response() == true).collect()
    }

    pub fn with_config(config: &Config) -> Middlewares {
        Middlewares {
            inner: Vec::default(),
            config: config.clone()
        }
    }

    pub async fn ensure_connected(&self) -> Result<Middlewares> {
        let mut middlewares = Middlewares::with_config(&self.config);

        for middleware in &self.inner {
            match &middleware.connection() {
                Some(_) => middlewares.insert_existing(&middleware),
                None => {
                    debug!("Trying to reconnect to {}", middleware.url());

                    let config_item = self.config.middlewares.iter()
                        .filter(|x| &x.url == middleware.url())
                        .collect::<Vec<&MiddlewareConfig>>();
                    let config_value = config_item.first().unwrap();
                    middlewares.insert(config_value).await?;
                }
            }
        }

        Ok(middlewares)
    }

    pub fn insert_existing(&mut self, item: &Middleware) -> () {
        self.inner.push(item.clone())
    }

    pub async fn insert(&mut self, middleware: &MiddlewareConfig) -> Result<()> {
        let connection = MiddlewareClient::connect(middleware.url.clone()).await;

        self.inner.push(match connection {
            Ok(val) => MiddlewareBuilder::new()
                .url(middleware.url.clone())
                .connection(Some(val))
                .request(middleware.request)
                .response(middleware.response)
                .timeout_millis(middleware.timeout_ms)
                .build(),
            Err(err) => {
                warn!("Error connecting to middleware [{}]: {}", middleware.url, err);

                MiddlewareBuilder::new()
                    .url(middleware.url.clone())
                    .connection(None)
                    .request(middleware.request)
                    .response(middleware.response)
                    .timeout_millis(middleware.timeout_ms)
                    .build()
            }
        });

        Ok(())
    }
}