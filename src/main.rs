mod api;
mod config;
mod definition;
mod git;
mod mailing;
mod repos;
mod runner;
mod secrets;

use crate::{mailing::MailSender, runner::Runner, secrets::SecretManager};
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

    debug!("config: {:#?}", &cfg);

    let secrets = cfg
        .secrets_file
        .as_ref()
        .map(SecretManager::new)
        .unwrap_or_else(|| Ok(SecretManager::empty()))?;

    let mailer = cfg
        .email
        .as_ref()
        .map(|mc| {
            MailSender::new(
                &mc.smtp_server,
                &mc.username,
                &mc.password,
                &mc.from_address,
            )
        })
        .transpose()?;

    let runner = Runner::new(secrets, mailer);

    if let Some(repo_dir) = cfg.repo_dir.clone() {
        let mut interval =
            tokio::time::interval(Duration::from_secs(cfg.index_interval_secs.unwrap_or(30)));
        let port = cfg.port;
        tokio::spawn(async move {
            loop {
                debug!("Indexing repos ...");
                if let Err(err) = repos::index(&repo_dir, port) {
                    error!("Repo indexing failed: {err}");
                }
                interval.tick().await;
            }
        });
    }

    api::run(&cfg, runner).await?;

    Ok(())
}
