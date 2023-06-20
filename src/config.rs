use figment::{
    providers::{Env, Format, Toml, Yaml},
    Figment,
};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub address: Option<String>,
    pub repo_dir: Option<PathBuf>,
    pub index_interval_secs: Option<u64>,
}

impl Config {
    pub fn parse() -> Result<Self, figment::Error> {
        Figment::new()
            .merge(Toml::file("minicd.toml"))
            .merge(Yaml::file("minicd.yaml"))
            .merge(Toml::file("/etc/minicd/config.toml"))
            .merge(Yaml::file("/etc/minicd/config.yaml"))
            .merge(Env::prefixed("MINICD_"))
            .extract()
    }
}
