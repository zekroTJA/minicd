mod api;
mod config;
mod repos;

use config::Config;
use env_logger::Env;
use log::{debug, error};
use std::{error::Error, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .try_init()
        .expect("failed initializing logger");

    let cfg = Config::parse()?;

    dbg!(&cfg);

    let mut interval =
        tokio::time::interval(Duration::from_secs(cfg.index_interval_secs.unwrap_or(30)));
    let repo_dir = cfg.repo_dir.clone();
    let port = cfg.port;
    tokio::spawn(async move {
        loop {
            debug!("Indexing repos ...");
            if let Err(err) = repos::index_repos(&repo_dir, port) {
                error!("Repo indexing failed: {err}");
            }
            interval.tick().await;
        }
    });

    api::run(cfg.address.as_deref().unwrap_or("0.0.0.0"), cfg.port).await?;

    Ok(())
}
