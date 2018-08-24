#[macro_use]
extern crate diesel;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate lettre;
extern crate lettre_email;
#[macro_use]
extern crate log;
//extern crate serde;
//#[macro_use]
//extern crate serde_derive;
extern crate tera;
extern crate tokio_threadpool;
//extern crate warp;

//#[macro_use]
//mod macros;

pub mod db;
mod errors;
mod mailer;
pub mod util;
//pub mod web;

pub use db::DB;
pub use errors::{Error, ErrorKind, Result};
pub use mailer::Mailer;
