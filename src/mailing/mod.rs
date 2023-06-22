pub mod error;

use self::error::{Error, Result};
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};

pub struct MailSender {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_address: String,
}

impl MailSender {
    pub fn new<S: Into<String>>(
        smtp_server: &str,
        username: S,
        password: S,
        from_address: S,
    ) -> Result<Self> {
        let creds = Credentials::new(username.into(), password.into());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_server)
            .map_err(Error::Transport)?
            .credentials(creds)
            .build();

        Ok(Self {
            mailer,
            from_address: from_address.into(),
        })
    }

    pub async fn send<S: Into<String>>(&self, to: &str, subject: S, body: S) -> Result<()> {
        let msg = Message::builder()
            .from(format!("minicd <{}>", self.from_address).parse()?)
            .to(to.parse()?)
            .subject("Test Email")
            .body(body.into())
            .map_err(Error::InvalidMessage)?;

        self.mailer.send(msg).await.map_err(Error::SendFailed)?;

        Ok(())
    }
}
