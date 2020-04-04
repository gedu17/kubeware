use serde_derive::Deserialize;

#[derive(Deserialize,Debug,Clone)]
pub struct Config {
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub backend: Backend,
    pub services: Vec<Service>
}

#[derive(Deserialize,Debug,Clone)]
pub struct Service {
    pub url: String,
    pub timeout_ms: Option<u32>,
    pub request: bool,
    pub response: bool
}

#[derive(Deserialize,Debug,Clone)]
pub struct Backend {
    pub url: String,
    pub timeout_ms: Option<u32>
}