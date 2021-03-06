extern crate dotenv;
#[macro_use]
extern crate failure;
extern crate futures;
#[macro_use]
extern crate log;
extern crate mailer;
#[macro_use]
extern crate structopt;
extern crate syslog;
extern crate tokio;
extern crate tokio_threadpool;
extern crate url;
extern crate warp;

use std::net::{SocketAddr, ToSocketAddrs};
use std::process::exit;
use std::sync::Arc;
use std::time::{Duration, Instant};

use failure::Error;
use futures::{Future, Stream};
use mailer::{log_err, routes, sweep, Mailer, DB};
use structopt::StructOpt;
use tokio::timer::Interval;
use tokio_threadpool::ThreadPool;
use url::Url;

fn main() {
    dotenv::dotenv().ok();
    let options = Options::from_args();
    options.start_logger();

    if let Err(err) = run(options) {
        log_err(err);
        exit(1);
    }
}

fn run(options: Options) -> Result<(), Error> {
    let serve_addr = options.serve_addr()?;
    let db = DB::connect(&options.database_url)?;
    let mailer = Mailer::new(
        options.smtp_addr,
        options.smtp_from,
        options.smtp_user,
        options.smtp_pass,
        options.smtp_reply_to,
    )?;

    let base_url = Arc::new(options.base_url);
    let routes = routes(db.clone(), options.auth_server, base_url.clone());
    let server = warp::serve(routes).bind(serve_addr);

    let thread_pool = ThreadPool::new();
    thread_pool.spawn(server);
    let sweeper = Interval::new(Instant::now(), Duration::from_secs(5 * 60))
        .map_err(Error::from)
        .for_each(move |_| {
            let fut = sweep(db.clone(), mailer.clone(), base_url.clone());
            Ok(thread_pool.spawn(fut.map_err(|e| log_err(e.into()))))
        })
        .map_err(log_err);

    tokio::run(sweeper);
    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "::structopt::clap::AppSettings::ColoredHelp"))]
struct Options {
    /// Turns off message output.
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// Increases the verbosity. Default verbosity is errors and warnings.
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,

    /// The URL of the authentication server to use, if any.
    #[structopt(short = "a", long = "auth-server", env = "AUTH_SERVER")]
    auth_server: Option<Url>,

    /// The base URL for unsubscribe links and template examples.
    #[structopt(short = "b", long = "base-url", env = "BASE_URL")]
    base_url: Url,

    /// The URL of the MySQL database.
    #[structopt(short = "d", long = "db", env = "DATABASE_URL")]
    database_url: String,

    /// The host to serve on.
    #[structopt(short = "h", long = "host", env = "HOST", default_value = "::")]
    host: String,

    /// The port to serve on.
    #[structopt(short = "p", long = "port", env = "PORT", default_value = "8001")]
    port: u16,

    /// The SMTP server to use.
    #[structopt(long = "smtp-addr", env = "SMTP_ADDR", default_value = "smtp.gmail.com")]
    smtp_addr: String,

    /// The SMTP From header to use.
    #[structopt(long = "smtp-from", env = "SMTP_FROM")]
    smtp_from: String,

    /// The SMTP username to use.
    #[structopt(long = "smtp-user", env = "SMTP_USER")]
    smtp_user: String,

    /// The SMTP password to use.
    #[structopt(long = "smtp-pass", env = "SMTP_PASS")]
    smtp_pass: String,

    /// The SMTP Reply-To header to use.
    #[structopt(long = "smtp-reply-to", env = "SMTP_REPLY_TO")]
    smtp_reply_to: Option<String>,

    /// The syslog server to send logs to.
    #[structopt(short = "s", long = "syslog-server", env = "SYSLOG_SERVER")]
    syslog_server: Option<String>,
}

impl Options {
    /// Get the address to serve on.
    fn serve_addr(&self) -> Result<SocketAddr, Error> {
        let addrs = (&self.host as &str, self.port)
            .to_socket_addrs()?
            .collect::<Vec<_>>();
        if addrs.is_empty() {
            bail!("No matching address exists")
        } else {
            Ok(addrs[0])
        }
    }

    /// Sets up logging as specified by the `-q`, `-s`, and `-v` flags.
    fn start_logger(&self) {
        if !self.quiet {
            let log_level = match self.verbose {
                0 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Info,
                2 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            };

            let r = if let Some(ref server) = self.syslog_server {
                syslog::init_tcp(
                    server,
                    "mail.acm.umn.edu".to_string(),
                    syslog::Facility::LOG_DAEMON,
                    log_level,
                )
            } else {
                // rifp https://github.com/Geal/rust-syslog/pull/38
                syslog::init(
                    syslog::Facility::LOG_DAEMON,
                    log_level,
                    Some("mail.acm.umn.edu"),
                )
            };

            if let Err(err) = r {
                error!("Warning: logging couldn't start: {}", err);
            }
        }
    }
}
