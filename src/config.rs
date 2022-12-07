use std::{net::SocketAddr, path::PathBuf};

use config as cfg;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_address: SocketAddr,
    pub docker_socket_path: String,
    pub container_name: String,
    pub template_path: PathBuf,
    pub valheim_server_address: SocketAddr,
    pub valheim_backups_path: PathBuf,
    pub valheim_backups_destination_path: PathBuf,
    pub valheim_server_restart_delay_seconds: u32,
    pub valheim_server_last_log_lines_count: u32,
    pub username: String,
    pub password: String,
}

impl Config {
    pub fn new() -> Result<Self, cfg::ConfigError> {
        cfg::Config::builder()
            .set_default("server_address", "0.0.0.0:9999")?
            .set_default("docker_socket_path", "/var/run/docker.sock")?
            .set_default("template_path", "templates/main.html")?
            .set_default("valheim_server_address", "127.0.0.1:2457")?
            .set_default("valheim_server_restart_delay_seconds", 60)?
            .set_default("valheim_server_last_log_lines_count", 100)?
            .add_source(cfg::File::with_name("config.toml").required(true))
            .build()?
            .try_deserialize()
    }
}
