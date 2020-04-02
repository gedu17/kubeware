use tonic::transport::Channel;
use crate::kubeware::proxy_client::ProxyClient;

#[derive(Clone)]
pub struct KubewareService {
    url: String,
    connection: Option<ProxyClient<Channel>>,
    pre_request: bool,
    post_request: bool
}

pub struct KubewareServiceBuilder {
    url: Option<String>,
    connection: Option<ProxyClient<Channel>>,
    pre_request: Option<bool>,
    post_request: Option<bool>
}

impl KubewareServiceBuilder {
    pub fn new() -> KubewareServiceBuilder {
        KubewareServiceBuilder {
            url: None,
            connection: None,
            pre_request: None,
            post_request: None
        }
    }

    pub fn url(mut self, url: String) -> KubewareServiceBuilder {
        self.url = Some(url);
        self
    }

    pub fn pre_request(mut self, enabled: bool) -> KubewareServiceBuilder {
        self.pre_request = Some(enabled);
        self
    }

    pub fn post_request(mut self, enabled: bool) -> KubewareServiceBuilder {
        self.post_request = Some(enabled);
        self
    }

    pub fn connection(mut self, connection: Option<ProxyClient<Channel>>) -> KubewareServiceBuilder {
        self.connection = connection;
        self
    }

    pub fn build(&self) -> KubewareService {
        KubewareService {
            url: self.url.as_ref().unwrap().to_string(),
            connection: self.connection.to_owned(),
            pre_request: self.pre_request.unwrap(),
            post_request: self.post_request.unwrap()
        }
    }
}


impl KubewareService {
    pub fn url(&self) -> &String {
        &self.url
    }

    #[allow(dead_code)]
    pub fn pre_request(&self) -> bool {
        self.pre_request
    }

    #[allow(dead_code)]
    pub fn post_request(&self) -> bool { self.post_request }

    pub fn connection(&self) -> &Option<ProxyClient<Channel>> {
        &self.connection
    }
}