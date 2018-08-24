#[macro_use]
extern crate diesel;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate lettre;
extern crate lettre_email;
#[macro_use]
extern crate log;
extern crate pulldown_cmark;
extern crate serde_json;
extern crate tera;
extern crate tokio_threadpool;
//extern crate warp;
extern crate url;

#[macro_use]
mod macros;

mod db;
mod errors;
mod mailer;
mod sweeper;
pub mod util;
//pub mod web;

pub use db::DB;
pub use errors::{Error, ErrorKind, Result};
pub use mailer::Mailer;
pub use sweeper::sweep;
