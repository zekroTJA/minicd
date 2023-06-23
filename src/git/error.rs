use std::{
    io,
    process::{ExitStatus, Output},
    string::FromUtf8Error,
};

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Command execution failed: {0}")]
    CommandExecution(io::Error),

    #[error("Command failed ({code}): {message}")]
    CommandStatus { code: ExitStatus, message: String },

    #[error("failed encoding output to UTF8: {0}")]
    OutputEncode(#[from] FromUtf8Error),
}

impl From<Output> for Error {
    fn from(value: Output) -> Self {
        Self::CommandStatus {
            code: value.status,
            message: String::from_utf8(value.stderr).unwrap_or_default(),
        }
    }
}
