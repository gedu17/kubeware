use serde_derive::Deserialize;

#[derive(Deserialize,Debug,Clone)]
pub struct Config {
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub log: Option<String>,
    pub backend: Backend,
    #[serde(rename = "middleware")]
    pub middlewares: Vec<MiddlewareConfig>
}

#[derive(Deserialize,Debug,Clone)]
pub struct MiddlewareConfig {
    pub url: String,
    pub timeout_ms: Option<u32>,
    pub request: bool,
    pub response: bool
}

#[derive(Deserialize,Debug,Clone)]
pub struct Backend {
    pub url: String,
    pub timeout_ms: Option<u32>,
    pub version: Option<HttpVersion>
}

#[derive(Deserialize,Debug,Clone)]
pub enum HttpVersion {
    HTTP,
    HTTP2
}