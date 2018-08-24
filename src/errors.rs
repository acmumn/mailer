use std::fmt::{Display, Formatter, Result as FmtResult};

use failure::{Backtrace, Context, Fail};

/// A convenient alias for Result.
pub type Result<T> = ::std::result::Result<T, Error>;

/// The kind of an application error.
#[derive(Debug, Fail)]
pub enum ErrorKind {
    /// Authentication was required but not provided.
    #[fail(display = "Authentication required")]
    AuthenticationRequired,

    /// Administrative privileges were needed, but the authenticated user does not have them.
    #[fail(display = "Insufficient privileges")]
    InsufficientPrivileges,

    /// Invalid data was attempted to be inserted into the database.
    #[fail(display = "{}", _0)]
    InvalidData(&'static str),

    /// No authentication server exists. This is the case when no authentication server URL is
    /// provided.
    #[fail(display = "No authentication server exists")]
    NoAuthServer,

    /// A template was attempted to be created, but it already exists.
    #[fail(display = "Template {:?} already exists", _0)]
    TemplateExists(String),

    /// An error from Diesel.
    #[fail(display = "Diesel error: {}", _0)]
    Diesel(::diesel::result::Error),

    /// An error from R2D2.
    #[fail(display = "R2D2 error: {}", _0)]
    R2D2(::diesel::r2d2::PoolError),

    /// An error from Lettre's SMTP transport.
    #[fail(display = "SMTP error: {}", _0)]
    Smtp(::lettre::smtp::error::Error),
}

impl From<::diesel::result::Error> for ErrorKind {
    fn from(err: ::diesel::result::Error) -> ErrorKind {
        ErrorKind::Diesel(err)
    }
}

impl From<::diesel::r2d2::PoolError> for ErrorKind {
    fn from(err: ::diesel::r2d2::PoolError) -> ErrorKind {
        ErrorKind::R2D2(err)
    }
}

impl From<::lettre::smtp::error::Error> for ErrorKind {
    fn from(err: ::lettre::smtp::error::Error) -> ErrorKind {
        ErrorKind::Smtp(err)
    }
}

/// An application error.
#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.inner.get_context()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        Display::fmt(&self.inner, f)
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl<E: Into<ErrorKind>> From<E> for Error {
    fn from(err: E) -> Error {
        Context::new(err.into()).into()
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner: inner }
    }
}
