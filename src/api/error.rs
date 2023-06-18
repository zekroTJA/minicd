use std::num::ParseIntError;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed parsing IP address: {0}")]
    ParseAddress(#[from] ParseIntError),
}
