use std::sync::{Arc, Mutex};

use failure::Error;
use futures::{future::poll_fn, prelude::*};
use lettre::{smtp::authentication::Credentials, EmailTransport, SmtpTransport};
use lettre_email::EmailBuilder;
use tera::Tera;
use tokio_threadpool::blocking;

use errors::Result;

/// The mailer. Cheaply clonable.
#[derive(Clone)]
pub struct Mailer {
    inner: Arc<MailerInner>,
    transport: Arc<Mutex<SmtpTransport>>,
}

struct MailerInner {
    from: String,
    reply_to: String,
}

impl Mailer {
    /// Creates a new `Mailer`.
    pub fn new(
        addr: String,
        from: String,
        user: String,
        pass: String,
        reply_to: Option<String>,
    ) -> Result<Mailer> {
        let transport = SmtpTransport::simple_builder(&addr)?
            .credentials(Credentials::new(user, pass))
            .smtp_utf8(true)
            .build();
        let reply_to = reply_to.unwrap_or_else(|| from.clone());
        Ok(Mailer {
            transport: Arc::new(Mutex::new(transport)),
            inner: Arc::new(MailerInner { from, reply_to }),
        })
    }

    fn send_mail(&self, email: EmailBuilder) -> impl Future<Item = (), Error = Error> {
        let transport = self.transport.clone();
        email
            .build()
            .map_err(Error::from)
            .into_future()
            .and_then(|email| {
                poll_fn(move || {
                    blocking(|| transport.lock().unwrap().send(&email).map_err(Error::from))
                        .map_err(|_| panic!("Emails must be sent inside a Tokio thread pool!"))
                }).and_then(|r| {
                    r.and_then(|r: ::lettre::smtp::response::Response| {
                        if r.is_positive() {
                            Ok(())
                        } else {
                            Err(format_err!("SMTP error: {}", r.code))
                        }
                    })
                })
            })
    }
}
