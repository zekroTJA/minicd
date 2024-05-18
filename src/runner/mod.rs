pub mod error;

use crate::{
    definition::{Definition, Job, JobState, Notify, NotifyTarget, Ref, ValueOrList},
    git::Repository,
    mailing::MailSender,
    secrets::SecretManager,
};
use error::{Error, Result};
use log::debug;
use reqwest::header::HeaderMap;
use run_script::ScriptOptions;
use std::{path::PathBuf, sync::Arc};
use temp_dir::TempDir;
use tokio::{fs::File, io::AsyncReadExt};

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

    pub async fn run(&self, remote: &str, reference: &str, reference_name: &str) -> Result<()> {
        let tmp_dir = TempDir::new().map_err(Error::TempDirCreationFailed)?;
        let tmp_dir_path = tmp_dir.path();

        let ref_typ: Ref = reference_name.parse()?;

        let mut repo = Repository::clone(remote, tmp_dir_path.to_str().unwrap_or_default())?;
        repo.checkout(reference)?;

        let def_path = tmp_dir_path.join(".minicd");
        if !def_path.exists() {
            return Err(Error::NoDefinitionFile);
        }

        let mut def_file = File::open(def_path).await?;
        let mut def_data = vec![];
        def_file.read_to_end(&mut def_data).await?;

        let def = Definition::parse(&def_data)?;

        for (job_id, job) in def.jobs {
            if let Some(on) = &job.on {
                if !on.matches(&ref_typ) {
                    debug!(
                        "Skipping job {job_id} because ref does not match \
                        ({on:?} != {ref_typ:?})",
                    );
                    continue;
                }
            }

            if matches!(job.await_result, Some(true)) {
                self.run_job(&job, tmp_dir_path.to_path_buf(), &def.name, &repo, &ref_typ)
                    .await?;
            } else {
                let tmp_dir_path_buf = tmp_dir_path.to_path_buf();
                let def_name = def.name.clone();
                let repo = repo.clone();
                let ref_typ = ref_typ.clone();
                let s = self.clone();
                tokio::spawn(async move {
                    if let Err(err) = s
                        .run_job(&job, tmp_dir_path_buf, &def_name, &repo, &ref_typ)
                        .await
                    {
                        log::error!("Async job failed: {err}");
                    }
                });
            }
        }

        Ok(())
    }

    async fn run_job(
        &self,
        job: &Job,
        dir: PathBuf,
        name: &str,
        repo: &Repository,
        ref_typ: &Ref,
    ) -> Result<()> {
        debug!("Starting job ...");
        if let Some(notifies) = job.get_notify(JobState::Start) {
            self.notify(&notifies, JobState::Start, name, repo, ref_typ, None)
                .await?;
        }

        match self.run_script(job, dir) {
            Ok(std_out) => {
                debug!("Job finished successful: {std_out}");
                if let Some(notifies) = job.get_notify(JobState::Success) {
                    self.notify(
                        &notifies,
                        JobState::Success,
                        name,
                        repo,
                        ref_typ,
                        Some(&std_out),
                    )
                    .await?;
                }
            }
            Err(err) => {
                debug!("Job failed: {err}");
                if let Some(notifies) = job.get_notify(JobState::Failure) {
                    self.notify(
                        &notifies,
                        JobState::Failure,
                        name,
                        repo,
                        ref_typ,
                        Some(&err.to_string()),
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    async fn notify(
        &self,
        notifies: &[&Notify],
        state: JobState,
        name: &str,
        repo: &Repository,
        ref_typ: &Ref,
        context: Option<&str>,
    ) -> Result<()> {
        for target in notifies.iter().flat_map(|n| &n.to) {
            match target {
                NotifyTarget::EMail { address } => {
                    let Some(mailer) = &self.0.mailer else {
                        log::warn!("mail notification: mailer has not been configured");
                        return Ok(());
                    };

                    let address = self.0.secrets.replace(address);
                    let subject = state.get_subject(name, ref_typ);
                    let body = state.get_body(name, ref_typ, repo.get_ref(), context);
                    mailer.send(&address, subject, body).await?;
                }
                NotifyTarget::WebHook {
                    url,
                    method,
                    headers,
                } => {
                    let url = self.0.secrets.replace(url);
                    let method = method.clone().unwrap_or_else(|| "GET".into()).parse()?;

                    let mut header_map = HeaderMap::new();
                    if let Some(headers) = headers {
                        header_map = headers.try_into().map_err(Error::WebhookInvlidHeaderMap)?;
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

    fn run_script(&self, job: &Job, dir: PathBuf) -> Result<String> {
        let mut options = ScriptOptions::new();
        options.working_directory = Some(dir);

        let env_vars = self
            .0
            .secrets
            .to_flat_map()
            .iter()
            .map(|(k, v)| (to_env_key(k), v.clone()))
            .collect();

        options.env_vars = Some(env_vars);

        if let Some(shell) = &job.shell {
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

        let (code, std_out, std_err) = run_script::run(&job.run, &vec![], &options)?;
        if code != 0 {
            return Err((code, std_err).into());
        }

        Ok(std_out)
    }
}

impl JobState {
    fn get_subject(&self, name: &str, ref_typ: &Ref) -> String {
        format!("{}: {name} @ {ref_typ}", self.get_subject_prefix())
    }

    fn get_body(&self, name: &str, ref_typ: &Ref, rf: &str, context: Option<&str>) -> String {
        match self {
            JobState::Start => format!(
                "A job has been started on project {name}.\n\
                \n\
                Reference: {ref_typ} ({rf})\n"
            ),
            JobState::Success => format!(
                "A job on project {name} has finished successful.\n\
                \n\
                Reference: {ref_typ} ({rf})\n\
                \n\
                Logs:\n\
                {}\n",
                context.unwrap_or_default()
            ),
            JobState::Failure => format!(
                "A job on project {name} has failed.\n\
                \n\
                Reference: {ref_typ} ({rf})\n\
                \n\
                Error:\n\
                {}\n",
                context.unwrap_or_default()
            ),
        }
    }

    fn get_subject_prefix(&self) -> &'static str {
        match self {
            Self::Start => "Job processing started",
            Self::Success => "Job finished successful",
            Self::Failure => "Job failed",
        }
    }
}

fn to_env_key(key: &str) -> String {
    format!("SECRETS_{}", key.to_uppercase().replace('.', "_"))
}
