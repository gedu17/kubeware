use crate::request_container::{RequestContainer, RequestContainerBuilder, ContainerState};
use hyper::{Request, Body, Response};
use hyper::header::{HeaderName, HeaderValue, CONTENT_LENGTH};
use crate::kubeware::{RequestRequest, RequestResponse, ResponseRequest, ResponseResponse};
use std::time::{Duration, Instant};
use hyper::http::method::Method;
use std::str::FromStr;
use crate::{KUBEWARE_TIME_HEADER, BACKEND_TIME_HEADER};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

pub struct ContainerHandler {
    container: RequestContainer,
    url: String,
    backend_elapsed: Option<Duration>,
    timer: Instant
}

impl ContainerHandler {
    pub fn state_set(&mut self, state: ContainerState) { self.container.state_set(state) }

    pub fn backend_elapsed_set(&mut self, elapsed: Duration) { self.backend_elapsed = Some(elapsed) }

    pub fn backend_elapsed(&mut self) -> Option<Duration> { self.backend_elapsed }

    pub fn timer(&mut self) -> Instant { self.timer }

    pub async fn new(request: Request<Body>, url: String) -> Result<ContainerHandler> {
        let (metadata, body) = request.into_parts();

        let request_container = RequestContainerBuilder::new()
            .method(metadata.method)
            .uri(metadata.uri)
            .version(metadata.version)
            .headers(metadata.headers)
            .body(hyper::body::to_bytes(body).await?);

        Ok(ContainerHandler {
            container: request_container.build(),
            url,
            backend_elapsed: Some(Duration::from_millis(0)),
            timer: Instant::now()
        })
    }

    pub fn handle_middleware_request(&mut self, response: &RequestResponse, stop: bool) -> Result<()> {
        self.container.remove_request_headers(&response.removed_headers);

        match stop {
            true => self.container.add_response_headers(&response.added_headers)?,
            false => self.container.add_request_headers(&response.added_headers)?
        };

        match response.status_code {
            Some(val) => self.container.status_code_set(val as u16),
            None => ()
        };

        match &response.body {
            Some(val) => self.container.body_set_string(val.clone()),
            None => ()
        };

        Ok(())
    }

    pub fn handle_middleware_response(&mut self, response: &ResponseResponse, stop: bool) -> Result<()> {

        // TODO: figure out how to return multiple set-cookie headers
        self.container.remove_response_headers(&response.removed_headers.clone());
        self.container.add_response_headers(&response.added_headers)?;

        match response.status_code {
            Some(val) => self.container.status_code_set(val as u16),
            None => if stop { self.container.status_code_set(500) } else { self.container.status_code_set(self.container.status_code().unwrap_or(500)) }
        };

        match &response.body {
            Some(val) => self.container.body_set_string(val.clone()),
            None => if stop { self.container.body_set_string(String::new()) } else { self.container.body_set_string(self.container.body()?) }
        };

        Ok(())
    }

    pub async fn handle_response(&mut self, response: Response<Body>) -> Result<()> {
        let (metadata, body) = response.into_parts();

        self.container.response_headers_set(metadata.headers.to_owned());
        self.container.status_code_set(metadata.status.as_u16());
        self.container.body_set_bytes(hyper::body::to_bytes(body).await?);

        Ok(())
    }

    pub fn into_middleware_request(&mut self) -> Result<RequestRequest> {
        Ok(RequestRequest {
            method: self.container.method(),
            uri: self.container.uri(),
            headers: self.container.headers(),
            body: self.container.request_body()?
        })
    }

    pub fn into_middleware_response(&mut self) -> Result<ResponseRequest> {
        Ok(ResponseRequest {
            method: self.container.method(),
            uri: self.container.uri(),
            request_headers: self.container.request_headers(),
            response_headers: self.container.response_headers(),
            request_body: self.container.request_body()?,
            response_body: self.container.response_body()?
        })
    }

    pub fn into_request(&mut self) -> Result<Request<Body>> {
        let mut request_builder = Request::builder()
            .method(Method::from_str(self.container.method().as_str())?)
            .uri(self.url.clone())
            .version(self.container.version());

        let headers_dict = request_builder.headers_mut().unwrap();

        for header in self.container.request_headers() {
            headers_dict.insert(HeaderName::from_lowercase(header.name.to_lowercase().as_bytes())?, HeaderValue::from_str(header.value.as_str())?);
        }

        headers_dict.remove(CONTENT_LENGTH);

        Ok(request_builder.body(self.container.request_body()?.into())?)
    }

    pub fn into_response(&mut self) -> Result<Response<Body>> {
        let mut response = Response::builder()
            .version(self.container.version())
            .status(self.container.status_code().unwrap_or(500))
            .header(KUBEWARE_TIME_HEADER, HeaderValue::from_str(&self.timer.elapsed().as_millis().to_string())?)
            .header(BACKEND_TIME_HEADER, HeaderValue::from_str(&self.backend_elapsed.unwrap().as_millis().to_string())?);

        let headers_dict = response.headers_mut().unwrap();
        let headers = self.container.response_headers().to_owned();

        for header in headers {
            headers_dict.insert(HeaderName::from_lowercase(header.name.to_lowercase().as_bytes())?, HeaderValue::from_str(header.value.as_str())?);
        }

        headers_dict.remove(CONTENT_LENGTH);

        info!("[{}] {} - {} | {} ms.", self.container.method(), self.container.uri(), self.container.status_code().unwrap_or(500), self.timer.elapsed().as_millis());

        Ok(response.body(self.container.body()?.into())?)
    }
}