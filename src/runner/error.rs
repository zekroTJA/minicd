use crate::{definition::RefParseError, git, mailing};
use reqwest::header::{InvalidHeaderName, InvalidHeaderValue};
use run_script::ScriptError;
use warp::http::method::InvalidMethod;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed creating temp directory: {0}")]
    TempDirCreationFailed(std::io::Error),

    #[error("git operation failed: {0}")]
    Git(#[from] git::error::Error),

    #[error("no definition file found")]
    NoDefinitionFile,

    #[error("failed reading definition file: {0}")]
    FailedReadingDefinitionFile(#[from] tokio::io::Error),

    #[error("failed deserializing definition file: {0}")]
    FailedDeserializingDefinitionFile(#[from] serde_yaml::Error),

    #[error("script execution failed: {0}")]
    Script(#[from] ScriptError),

    #[error("script failed with non-zero exit code {exit_code}: {std_err}")]
    ScriptNonZeroExitCode { exit_code: i32, std_err: String },

    #[error("notification webhook invalid method: {0}")]
    WebhookInvlidMethod(#[from] InvalidMethod),

    #[error("notification webhook invalid header name: {0}")]
    WebhookInvlidHeaderName(#[from] InvalidHeaderName),

    #[error("notification webhook invalid header value: {0}")]
    WebhookInvlidHeaderValue(#[from] InvalidHeaderValue),

    #[error("notification webhook request failed: {0}")]
    WebhookError(#[from] reqwest::Error),

    #[error("notification mail send failed: {0}")]
    MailError(#[from] mailing::error::Error),

    #[error(transparent)]
    InvalidReferenceName(#[from] RefParseError),
}

impl From<(i32, String)> for Error {
    fn from(value: (i32, String)) -> Self {
        let (exit_code, std_err) = value;
        Self::ScriptNonZeroExitCode { exit_code, std_err }
    }
}
