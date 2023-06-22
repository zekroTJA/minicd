pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed creating mailer: {0}")]
    Transport(lettre::transport::smtp::Error),

    #[error("failed parsing address: {0}")]
    InvalidAddress(#[from] lettre::address::AddressError),

    #[error("failed building message: {0}")]
    InvalidMessage(lettre::error::Error),

    #[error("failed sending message: {0}")]
    SendFailed(lettre::transport::smtp::Error),
}
