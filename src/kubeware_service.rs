use tonic::transport::Channel;
use crate::kubeware::middleware_client::MiddlewareClient;
use std::time::Duration;
use crate::DEFAULT_TIMEOUT_MILLIS;

#[derive(Clone)]
pub struct KubewareService {
    url: String,
    connection: Option<MiddlewareClient<Channel>>,
    timeout: Duration,
    request: bool,
    response: bool
}

pub struct KubewareServiceBuilder {
    url: Option<String>,
    connection: Option<MiddlewareClient<Channel>>,
    request: Option<bool>,
    response: Option<bool>,
    timeout_millis: Option<u32>
}

impl KubewareServiceBuilder {
    pub fn new() -> KubewareServiceBuilder {
        KubewareServiceBuilder {
            url: None,
            connection: None,
            request: None,
            response: None,
            timeout_millis: None
        }
    }

    pub fn url(mut self, url: String) -> KubewareServiceBuilder {
        self.url = Some(url);
        self
    }

    pub fn request(mut self, enabled: bool) -> KubewareServiceBuilder {
        self.request = Some(enabled);
        self
    }

    pub fn response(mut self, enabled: bool) -> KubewareServiceBuilder {
        self.response = Some(enabled);
        self
    }

    pub fn connection(mut self, connection: Option<MiddlewareClient<Channel>>) -> KubewareServiceBuilder {
        self.connection = connection;
        self
    }

    pub fn timeout_millis(mut self, ms: Option<u32>) -> KubewareServiceBuilder {
        match ms {
            Some(val) => self.timeout_millis = Some(val),
            _ => self.timeout_millis = Some(DEFAULT_TIMEOUT_MILLIS)
        }

        self
    }

    pub fn build(&self) -> KubewareService {
        KubewareService {
            url: self.url.as_ref().unwrap().to_string(),
            connection: self.connection.to_owned(),
            request: self.request.unwrap_or(false),
            response: self.response.unwrap_or(false),
            timeout:  Duration::from_millis(self.timeout_millis.unwrap() as u64)
        }
    }
}


impl KubewareService {
    pub fn url(&self) -> &String { &self.url }

    #[allow(dead_code)]
    pub fn request(&self) -> bool {  self.request }

    #[allow(dead_code)]
    pub fn response(&self) -> bool { self.response }

    pub fn connection(&self) -> &Option<MiddlewareClient<Channel>> { &self.connection }

    pub fn timeout(&self) -> Duration { self.timeout }
}