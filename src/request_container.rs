use std::time::Instant;
use core::time::Duration;
use hyper::{Uri, Method, Version, Body, Request, Response};
use bytes::Bytes;
use crate::kubeware::{PreRequest, PreResponse, RequestHeader, PostRequest, PostResponse};
use hyper::http::header::{HeaderName, HeaderValue, HeaderMap};
use hyper::header::CONTENT_LENGTH;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

const KUBEWARE_TIME_HEADER: &str = "x-kubeware-time";
const BACKEND_TIME_HEADER: &str  = "x-backend-time";

#[derive(Default)]
pub struct HttpContainer {
    pub headers: HeaderMap,
    pub body: Bytes
}
// TODO: implement builder and make some fields optional
pub struct RequestContainer {
    method: Method,
    path: Uri,
    version: Version,
    status_code: u16,
    request: HttpContainer,
    response: HttpContainer,
    backend_elapsed: Duration,
    timer: Instant
}

impl RequestContainer {
    pub fn backend_elapsed(&mut self, elapsed: Duration) {
        self.backend_elapsed = elapsed
    }

    pub async fn from_request(request: Request<Body>) -> Result<RequestContainer> {
        let (metadata, body) = request.into_parts();

        Ok(RequestContainer {
            method: metadata.method.to_owned(),
            path: metadata.uri.to_owned(),
            version: metadata.version.to_owned(),
            status_code: 500,
            request: HttpContainer {
                headers: metadata.headers.to_owned(),
                body: hyper::body::to_bytes(body).await?
            },
            response: HttpContainer {
                headers: HeaderMap::default(),
                body: Bytes::default(),
            },
            backend_elapsed: Duration::from_millis(0),
            timer: Instant::now()
        })
    }

    pub fn parse_pre_request(&mut self, response: &PreResponse) -> Result<()> {

        match response.status_code == 0 {
            true => (),
            false => self.status_code = response.status_code as u16
        };

        // Remove headers
        for header in &response.removed_headers {
            self.request.headers.remove(header);
        }

        // Add headers
        for header in &response.added_headers {
            self.request.headers.insert(HeaderName::from_lowercase(header.name.to_lowercase().as_bytes())?,
                                            HeaderValue::from_str(&header.value)?);
        }

        match response.body.is_empty() {
            true => (),
            false => self.request.body = Bytes::from(response.body.clone())
        };

        Ok(())
    }

    pub fn parse_post_request(&mut self, response: &PostResponse) -> Result<()> {

        match response.status_code == 0 {
            true => (),
            false => self.status_code = response.status_code as u16
        };

        // Remove headers
        for header in &response.removed_headers {
            self.response.headers.remove(header);
        }

        // TODO: figure out how to return multiple set-cookie headers

        // Add headers
        for header in &response.added_headers {
            self.response.headers.append(HeaderName::from_lowercase(header.name.to_lowercase().as_bytes())?,
                                         HeaderValue::from_str(&header.value)?);
        }

        match response.body.is_empty() {
            true => (),
            false => self.response.body = Bytes::from(response.body.clone())
        };

        Ok(())
    }

    pub async fn parse_be_response(&mut self, response: Response<Body>) -> Result<()> {
        let (metadata, body) = response.into_parts();

        self.response.headers = metadata.headers.to_owned();
        self.status_code = metadata.status.as_u16();
        self.response.body = hyper::body::to_bytes(body).await?;

        Ok(())
    }

    pub fn generate_pre_request(&mut self) -> Result<PreRequest> {
        Ok(PreRequest {
            method: self.method.to_string(),
            path: self.path.to_string(),
            headers: self.request.headers.iter().map(|x| RequestHeader {
                name: x.0.to_string(),
                value: x.1.to_str().unwrap().to_string()
            }).collect::<Vec<RequestHeader>>(),
            body: std::str::from_utf8(self.request.body.as_ref())?.to_string()
        })
    }

    pub fn generate_post_request(&mut self) -> Result<PostRequest> {
        Ok(PostRequest {
            method: self.method.to_string(),
            path: self.path.to_string(),
            request_headers: self.request.headers.iter().map(|x| RequestHeader {
                name: x.0.to_string(),
                value: x.1.to_str().unwrap().to_string()
            }).collect(),
            response_headers: self.response.headers.iter().map(|x| RequestHeader {
                name: x.0.to_string(),
                value: x.1.to_str().unwrap().to_string()
            }).collect(),
            request_body: std::str::from_utf8(self.request.body.as_ref())?.to_string(),
            response_body: std::str::from_utf8(self.response.body.as_ref())?.to_string()
        })
    }

    pub fn request_from_pre_request(&mut self, uri: String) -> Result<Request<Body>> {
        let mut request_builder = Request::builder()
            .method(self.method.clone())
            .uri(uri)
            .version(self.version.clone());

        let headers_dict = request_builder.headers_mut().unwrap();

        for header in &self.request.headers {
            headers_dict.insert(header.0, header.1.clone());
        }

        headers_dict.insert(CONTENT_LENGTH, self.request.body.len().into());

        Ok(request_builder.body(self.request.body.clone().into())?)
    }

    pub fn response_from_response(&mut self) -> Result<Response<Body>> {
        let mut response = Response::builder()
            .version(self.version)
            .status(self.status_code)
            .header(KUBEWARE_TIME_HEADER, HeaderValue::from_str(&self.timer.elapsed().as_millis().to_string())?)
            .header(BACKEND_TIME_HEADER, HeaderValue::from_str(&self.backend_elapsed.as_millis().to_string())?);

        let headers_dict = response.headers_mut().unwrap();

        for header in &self.response.headers {
            headers_dict.insert(header.0, header.1.clone());
        }

        headers_dict.insert(CONTENT_LENGTH, self.response.body.len().into());

        info!("[{}] {} - {} | {} ms.", self.method, self.path, self.status_code, self.timer.elapsed().as_millis());

        Ok(response.body(self.response.body.clone().into()).unwrap())
    }

    pub fn response_from_pre_request(&mut self) -> Result<Response<Body>> {
        let mut response = Response::builder()
            .status(self.status_code)
            .version(self.version)
            .header(KUBEWARE_TIME_HEADER, HeaderValue::from_str(&self.timer.elapsed().as_millis().to_string())?)
            .header(BACKEND_TIME_HEADER, HeaderValue::from_str(&self.backend_elapsed.as_millis().to_string())?);

        let headers_dict = response.headers_mut().unwrap();

        for header in &self.request.headers {
            headers_dict.insert(header.0, header.1.clone());
        }

        headers_dict.insert(CONTENT_LENGTH, self.request.body.len().into());

        info!("[{}] {} - {} | {} ms.", self.method, self.path, self.status_code, self.timer.elapsed().as_millis());

        Ok(response.body(self.request.body.clone().into()).unwrap())
    }

    pub fn response_from_post_request(&mut self) -> Result<Response<Body>> {
        let mut response = Response::builder()
            .status(self.status_code)
            .version(self.version)
            .header(KUBEWARE_TIME_HEADER, HeaderValue::from_str(&self.timer.elapsed().as_millis().to_string())?)
            .header(BACKEND_TIME_HEADER, HeaderValue::from_str(&self.backend_elapsed.as_millis().to_string())?);

        let headers_dict = response.headers_mut().unwrap();

        for header in &self.response.headers {
            headers_dict.insert(header.0, header.1.clone());
        }

        headers_dict.insert(CONTENT_LENGTH, self.response.body.len().into());

        info!("[{}] {} - {} | {} ms.", self.method, self.path, self.status_code, self.timer.elapsed().as_millis());

        Ok(response.body(self.response.body.clone().into()).unwrap())
    }
}