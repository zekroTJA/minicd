use run_script::ScriptError;

use crate::git;

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
}

impl From<(i32, String)> for Error {
    fn from(value: (i32, String)) -> Self {
        let (exit_code, std_err) = value;
        Self::ScriptNonZeroExitCode { exit_code, std_err }
    }
}
