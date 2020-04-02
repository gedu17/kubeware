mod request_container;
mod services;
mod config;
mod tower_service;
mod request_handler;
mod kubeware_service;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use crate::config::Config;
use std::fs::{File};
use std::env::{var};
use std::path::{Path};
use std::io::Read;
use hyper::{Client, Server};
use std::net::ToSocketAddrs;
use std::sync::{Arc};
use std::sync::Mutex;

use services::{Services};
use crate::tower_service::Builder;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

const LOOPBACK: &str = "127.0.0.1";
const PORT: u16 = 17000;

pub mod kubeware {
    tonic::include_proto!("kubeware");
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::try_init_timed()?;

    let config_file = match var("CONFIG_FILE") {
        Ok(value) => File::open(Path::new(&value)),
        Err(_) => File::open(Path::new("config.toml"))
    };

    let mut config_content = String::new();
    config_file?.read_to_string(&mut config_content)?;

    let config: Config = toml::from_str(config_content.as_str())?;
    let address = [config.ip.clone().unwrap_or(LOOPBACK.to_string()), config.port.clone().unwrap_or(PORT).to_string()]
        .join(":")
        .to_socket_addrs()?
        .next()
        .unwrap();

    let mut services = Services::with_config(&config);

    for service in &config.services {
        services.insert(service).await?;
    }

    let binded_server = Server::bind(&address).serve(Builder {
        http_client: Client::new(),
        config,
        mutex: Mutex::new(false),
        services: Arc::new(services)
    });

    let server = binded_server.with_graceful_shutdown(sigterm_signal());

    if let Err(err) = server.await {
        error!("Fatal error: {:?}", err);
    }

    Ok(())
}

async fn sigterm_signal() {
    tokio::signal::ctrl_c().await
        .expect("failed to install SIGTERM handler");
}