pub mod error;

use crate::{
    definition::{Definition, Event, Notify, NotifyTarget, ValueOrList},
    git,
    mailing::MailSender,
    secrets::SecretManager,
};
use error::{Error, Result};
use log::debug;
use run_script::ScriptOptions;
use std::{path::PathBuf, sync::Arc};
use temp_dir::TempDir;
use tokio::{fs::File, io::AsyncReadExt};

enum JobOutcome {
    Start,
    Success,
    Failure,
}

impl JobOutcome {
    fn get_subject(&self) -> &'static str {
        match self {
            Self::Start => "Job processing started",
            Self::Success => "Job finished successful",
            Self::Failure => "Job failed",
        }
    }
}

struct RunnerData {
    secrets: SecretManager,
    mailer: Option<MailSender>,
}

pub struct Runner(Arc<RunnerData>);

impl Clone for Runner {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Runner {
    pub fn new(secrets: SecretManager, mailer: Option<MailSender>) -> Self {
        Self(Arc::new(RunnerData { secrets, mailer }))
    }

    pub async fn run(&self, remote: &str, reference: &str) -> Result<()> {
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
            let s = self.clone();
            tokio::spawn(async move {
                if let Err(err) = s.run_job(&def, tmp_dir_path_buf).await {
                    log::error!("Async job failed: {err}");
                }
            });
        } else {
            self.run_job(&def, tmp_dir_path.to_path_buf()).await?;
        }

        Ok(())
    }

    async fn run_job(&self, def: &Definition, dir: PathBuf) -> Result<()> {
        debug!("Starting job ...");
        if let Some(notifies) = def.get_notify(Event::Start) {
            self.notify(&notifies, JobOutcome::Start).await?;
        }

        match self.run_script(def, dir) {
            Ok(std_out) => {
                debug!("Job finished successful: {std_out}");
                if let Some(notifies) = def.get_notify(Event::Success) {
                    self.notify(&notifies, JobOutcome::Success).await?;
                }
            }
            Err(err) => {
                debug!("Job failed: {err}");
                if let Some(notifies) = def.get_notify(Event::Failure) {
                    self.notify(&notifies, JobOutcome::Failure).await?;
                }
            }
        }

        Ok(())
    }

    async fn notify(&self, notifies: &[&Notify], outcome: JobOutcome) -> Result<()> {
        for target in notifies.iter().flat_map(|n| &n.to) {
            match target {
                NotifyTarget::EMail { address } => {
                    let Some(mailer) = &self.0.mailer else {
                        log::warn!("mail notification: mailer has not been configured");
                        return Ok(())
                    };

                    mailer.send(address, "", "").await?;
                }
                NotifyTarget::WebHook {
                    url: _,
                    method: _,
                    headers: _,
                } => {
                    log::error!("webhook notifications are not implemented yet!")
                    // let url = self.0.secrets.replace(url);
                    // let method = method.clone().unwrap_or_else(|| "GET".into()).parse()?;

                    // let mut header_map = HeaderMap::new();
                    // if let Some(headers) = headers {
                    //     for (k, v) in headers {
                    //         let v = self.0.secrets.replace(v);
                    //         header_map.insert(HeaderName::from_str(k)?, HeaderValue::from_str(&v)?);
                    //     }
                    // }

                    // debug!("Sending notification request ...");
                    // reqwest::Client::default()
                    //     .request(method, url)
                    //     .headers(header_map)
                    //     .send()
                    //     .await?
                    //     .error_for_status()?;
                }
            }
        }
        Ok(())
    }

    fn run_script(&self, def: &Definition, dir: PathBuf) -> Result<String> {
        let mut options = ScriptOptions::new();
        options.working_directory = Some(dir);

        options.env_vars = Some(self.0.secrets.to_flat_map());

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
}
