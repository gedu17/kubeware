use crate::kubeware::middleware_client::MiddlewareClient;
use crate::config::{Service, Config};
use crate::kubeware_service::{KubewareService, KubewareServiceBuilder};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[derive(Clone)]
pub struct Services {
    inner: Vec<KubewareService>,
    config: Config
}

impl Services {
    pub fn all(&self) -> &Vec<KubewareService> {
        &self.inner
    }

    pub fn request(&self) -> Vec<&KubewareService> {
        self.inner.iter().filter(|x| x.request() == true).collect()
    }

    pub fn response(&self) -> Vec<&KubewareService> {
        self.inner.iter().filter(|x| x.response() == true).collect()
    }

    pub fn with_config(config: &Config) -> Services {
        Services {
            inner: Vec::default(),
            config: config.clone()
        }
    }

    pub async fn ensure_connected(&self) -> Result<Services> {
        let mut services = Services::with_config(&self.config);

        for service in &self.inner {
            match &service.connection() {
                Some(_) => services.insert_existing(&service),
                None => {
                    debug!("Trying to reconnect to {}", service.url());

                    let config_item = self.config.services.iter()
                        .filter(|x| &x.url == service.url())
                        .collect::<Vec<&Service>>();
                    let config_value = config_item.first().unwrap();
                    services.insert(config_value).await?;
                }
            }
        }

        Ok(services)
    }

    pub fn insert_existing(&mut self, item: &KubewareService) -> () {
        self.inner.push(item.clone())
    }

    pub async fn insert(&mut self, service: &Service) -> Result<()> {
        let connection = MiddlewareClient::connect(service.url.clone()).await;

        self.inner.push(match connection {
            Ok(val) => KubewareServiceBuilder::new()
                .url(service.url.clone())
                .connection(Some(val))
                .request(service.request)
                .response(service.response)
                .build(),
            Err(err) => {
                warn!("Error connecting to service [{}]: {}", service.url, err);

                KubewareServiceBuilder::new()
                    .url(service.url.clone())
                    .connection(None)
                    .request(service.request)
                    .response(service.response)
                    .build()
            }
        });

        Ok(())
    }
}