pub mod error;

use std::{path::PathBuf, str::FromStr, sync::Arc};

use crate::{
    definition::{Definition, Event, Notify, NotifyTarget, ValueOrList},
    git,
    secrets::SecretManager,
};
use error::{Error, Result};
use log::debug;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use run_script::ScriptOptions;
use temp_dir::TempDir;
use tokio::{fs::File, io::AsyncReadExt};

pub async fn run(remote: &str, reference: &str, secrets: Arc<SecretManager>) -> Result<()> {
    let tmp_dir = TempDir::new().map_err(Error::TempDirCreationFailed)?;
    let tmp_dir_path = tmp_dir.path();

    git::clone(remote, tmp_dir_path.to_str().unwrap_or_default())?;
    git::checkout(reference, tmp_dir_path.to_str().unwrap_or_default())?;

    let def_path = tmp_dir_path.join(".minicd");
    if !def_path.exists() {
        return Err(Error::NoDefinitionFile);
    }

    let mut def_file = File::open(def_path).await?;
    let mut def_data = vec![];
    def_file.read_to_end(&mut def_data).await?;

    let def = Definition::parse(&def_data)?;

    if matches!(def.await_result, Some(true)) {
        let tmp_dir_path_buf = tmp_dir_path.to_path_buf();
        tokio::spawn(async move {
            if let Err(err) = run_job(&def, tmp_dir_path_buf, secrets).await {
                log::error!("Async job failed: {err}");
            }
        });
    } else {
        run_job(&def, tmp_dir_path.to_path_buf(), secrets).await?;
    }

    Ok(())
}

async fn run_job(def: &Definition, dir: PathBuf, secrets: Arc<SecretManager>) -> Result<()> {
    debug!("Starting job ...");
    if let Some(notifies) = def.get_notify(Event::Start) {
        notify(&notifies, &secrets).await?;
    }

    match run_script(def, dir, &secrets) {
        Ok(std_out) => {
            debug!("Job finished successful: {std_out}");
            if let Some(notifies) = def.get_notify(Event::Success) {
                notify(&notifies, &secrets).await?;
            }
        }
        Err(err) => {
            debug!("Job failed: {err}");
            if let Some(notifies) = def.get_notify(Event::Failure) {
                notify(&notifies, &secrets).await?;
            }
        }
    }

    Ok(())
}

async fn notify(notifies: &[&Notify], secrets: &SecretManager) -> Result<()> {
    for target in notifies.iter().flat_map(|n| &n.to) {
        match target {
            NotifyTarget::EMail { address: _ } => {
                log::error!("e-mail notifications are not implemented yet!")
            }
            NotifyTarget::WebHook {
                url,
                method,
                headers,
            } => {
                let url = secrets.replace(url);
                let method = method.clone().unwrap_or_else(|| "GET".into()).parse()?;

                let mut header_map = HeaderMap::new();
                if let Some(headers) = headers {
                    for (k, v) in headers {
                        let v = secrets.replace(v);
                        header_map.insert(HeaderName::from_str(k)?, HeaderValue::from_str(&v)?);
                    }
                }

                debug!("Sending notification request ...");
                reqwest::Client::default()
                    .request(method, url)
                    .headers(header_map)
                    .send()
                    .await?
                    .error_for_status()?;
            }
        }
    }
    Ok(())
}

fn run_script(def: &Definition, dir: PathBuf, secrets: &SecretManager) -> Result<String> {
    let mut options = ScriptOptions::new();
    options.working_directory = Some(dir);

    options.env_vars = Some(secrets.to_flat_map());

    if let Some(shell) = &def.shell {
        match shell {
            ValueOrList::Value(v) => options.runner = Some(v.to_owned()),
            ValueOrList::List(v) => {
                options.runner = v.first().cloned();
                if v.len() > 1 {
                    options.runner_args = Some(v.iter().skip(1).cloned().collect());
                }
            }
        }
    }

    let (code, std_out, std_err) = run_script::run(&def.run, &vec![], &options)?;
    if code != 0 {
        return Err((code, std_err).into());
    }

    Ok(std_out)
}