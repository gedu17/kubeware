use hyper::{Uri, Method, Version, HeaderMap};
use bytes::Bytes;
use crate::kubeware::{Header};
use hyper::header::{HeaderName, HeaderValue};
use crate::request_container::ContainerState::MiddlewareRequest;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[derive(Default)]
pub struct HttpContainer {
    pub headers: HeaderMap,
    pub body: Bytes
}

pub enum ContainerState {
    MiddlewareRequest,
    #[allow(dead_code)]
    Response,
    #[allow(dead_code)]
    MiddlewareResponse
}

pub struct RequestContainer {
    method: Method,
    uri: Uri,
    version: Version,
    status_code: Option<u16>,
    request: HttpContainer,
    response: HttpContainer,
    state: ContainerState
}

pub struct RequestContainerBuilder {
    method: Option<Method>,
    uri: Option<Uri>,
    version: Option<Version>,
    request: HttpContainer
}

impl RequestContainerBuilder {
    pub fn new() -> RequestContainerBuilder {
        RequestContainerBuilder {
            method: None,
            uri: None,
            version: None,
            request: HttpContainer {
                headers: HeaderMap::default(),
                body: Bytes::default()
            }
        }
    }

    pub fn method(mut self, method: Method) -> RequestContainerBuilder {
        self.method = Some(method);
        self
    }

    pub fn uri(mut self, uri: Uri) -> RequestContainerBuilder {
        self.uri = Some(uri);
        self
    }

    pub fn version(mut self, version: Version) -> RequestContainerBuilder {
        self.version = Some(version);
        self
    }

    pub fn headers(mut self, headers: HeaderMap) -> RequestContainerBuilder {
        self.request.headers = headers;
        self
    }

    pub fn body (mut self, body: Bytes) -> RequestContainerBuilder {
        self.request.body = body;
        self
    }

    pub fn build(self) -> RequestContainer {
        RequestContainer {
            method: self.method.clone().unwrap(),
            uri: self.uri.clone().unwrap(),
            version: self.version.unwrap(),
            status_code: None,
            request: self.request.into(),
            response: HttpContainer {
                headers: HeaderMap::default(),
                body: Bytes::default()
            },
            state: MiddlewareRequest
        }
    }
}

impl RequestContainer {
    pub fn state_set(&mut self, state: ContainerState) { self.state = state }

    pub fn state(&mut self) -> &ContainerState { &self.state }

    pub fn response_headers_set(&mut self, headers: HeaderMap) { self.response.headers = headers }

    pub fn status_code_set(&mut self, status_code: u16) { self.status_code = Some(status_code) }

    pub fn add_headers(&mut self, headers:&Vec<Header>) -> Result<()> {
        let mut container = match self.state {
            MiddlewareRequest => self.request.headers.to_owned(),
            _ => self.response.headers.to_owned()
        };

        for header in headers {
            container.insert(HeaderName::from_lowercase(header.name.to_lowercase().as_bytes())?,
                       HeaderValue::from_str(&header.value)?);
        }

        Ok(())
    }

    pub fn body_set_string(&mut self, body: String) {
        match self.state {
            MiddlewareRequest => self.request.body = Bytes::from(body),
            _ => self.response.body = Bytes::from(body)
        }
    }

    pub fn body_set_bytes(&mut self, body: Bytes) {
        match self.state {
            MiddlewareRequest => self.request.body = body,
            _ => self.response.body = body
        }
    }

    pub fn remove_headers(&mut self, headers: &Vec<String>) {
        let mut container = match self.state {
            MiddlewareRequest => self.request.headers.to_owned(),
            _ => self.response.headers.to_owned()
        };

        for header in headers {
            container.remove(header);
        }
    }

    pub fn method (&self) -> String { self.method.to_string() }

    pub fn version (&self) -> Version { self.version }

    pub fn uri (&self) -> String { self.uri.to_string() }

    pub fn headers (&self) -> Vec<Header> {
        match self.state {
            MiddlewareRequest => self.request_headers(),
            _ => self.response_headers()
        }
    }

    pub fn request_headers (&self) -> Vec<Header> {
        self.request.headers.iter().map(|x| Header {
            name: x.0.to_string(),
            value: x.1.to_str().unwrap().to_string()
        }).collect::<Vec<Header>>()
    }

    pub fn response_headers (&self) -> Vec<Header> {
        self.response.headers.iter().map(|x| Header {
            name: x.0.to_string(),
            value: x.1.to_str().unwrap().to_string()
        }).collect::<Vec<Header>>()
    }

    pub fn request_body (&self) -> Result<String> { Ok(std::str::from_utf8(self.request.body.as_ref())?.to_string()) }

    pub fn response_body (&self) -> Result<String> { Ok(std::str::from_utf8(self.response.body.as_ref())?.to_string()) }

    pub fn status_code (&self) -> Option<u16> { self.status_code }

    pub fn body (&self) -> Result<String> {
        match self.state {
            MiddlewareRequest => self.request_body(),
            _ => self.response_body()
        }
    }

}