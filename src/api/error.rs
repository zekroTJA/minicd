use std::{num::ParseIntError, str::Utf8Error};

use warp::reject::Reject;

use crate::runner;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed parsing IP address: {0}")]
    ParseAddress(#[from] ParseIntError),
}

#[derive(thiserror::Error, Debug)]
pub enum ResponseError {
    #[error("invalid body format: {0}")]
    InvalidBodyFormat(Utf8Error),

    #[error("missing body args: {0}")]
    MissingBodyArgs(&'static str),

    #[error("run failed: {0}")]
    RunFailed(runner::error::Error),
}

impl Reject for ResponseError {}
