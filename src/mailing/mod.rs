pub mod error;

use self::error::{Error, Result};
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, Message, SmtpTransport,
    Transport,
};

pub struct MailSender {
    mailer: SmtpTransport,
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

        let mailer = SmtpTransport::relay(smtp_server)
            .map_err(Error::Transport)?
            .credentials(creds)
            .build();

        Ok(Self {
            mailer,
            from_address: from_address.into(),
        })
    }

    pub fn send<S: Into<String>>(&self, to: &str, subject: S, body: S) -> Result<()> {
        let msg = Message::builder()
            .from(format!("minicd <{}>", self.from_address).parse()?)
            .to(to.parse()?)
            .subject("Test Email")
            .body(body.into())
            .map_err(Error::InvalidMessage)?;

        self.mailer.send(&msg).map_err(Error::SendFailed)?;

        Ok(())
    }
}
