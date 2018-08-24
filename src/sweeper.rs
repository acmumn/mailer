use std::sync::Arc;

use futures::{prelude::*, stream::poll_fn};
use serde_json::{self, Value};
use url::Url;

use {util::log_err, Error, Mailer, DB};

/// Sweeps all unsent emails from the database (by sending them).
pub fn sweep(db: DB, mailer: Mailer, base_url: Arc<Url>) -> impl Future<Item = (), Error = Error> {
    info!("Started sweeping.");
    get_all_unsent(db.clone())
        .and_then(
            move |(id, mailing_list_id, template_id, to_addr, subject, data)| {
                let base_url = base_url.clone();
                let db2 = db.clone();
                let mailer2 = mailer.clone();

                db.load_template(template_id)
                    .join(serde_json::from_str::<Value>(&data).map_err(Error::from))
                    .and_then(move |(render, data)| {
                        let mut unsubscribe = base_url.join("unsubscribe")?;
                        unsubscribe
                            .query_pairs_mut()
                            .clear()
                            .append_pair("email", &to_addr)
                            .append_pair("list", &mailing_list_id.to_string());
                        render(context! {data: data, unsubscribe: unsubscribe.to_string()})
                            .map(|body| (to_addr, body))
                    })
                    .and_then(move |(to_addr, body)| mailer2.send_mail(to_addr, subject, body))
                    .and_then(move |()| db2.set_email_done(id))
            },
        )
        .then(Ok)
        .fold((0, 0), |(succ, fail), r| -> Result<_, Error> {
            Ok(match r {
                Ok(()) => (succ + 1, fail),
                Err(e) => {
                    log_err(e.into());
                    (succ, fail + 1)
                }
            })
        })
        .map(|(succ, fail)| {
            use log::Level;
            if succ + fail > 0 {
                let level = if fail == 0 { Level::Info } else { Level::Error };
                log!(
                    level,
                    "Sweeping finished ({} successes, {} failures).",
                    succ,
                    fail
                );
            }
        })
}

fn get_all_unsent(
    db: DB,
) -> impl Stream<Item = (u32, u32, u32, String, String, String), Error = Error> {
    let mut fut = db.get_next_to_send();
    poll_fn(move || loop {
        match fut.poll() {
            Ok(Async::Ready(Some(data))) => {
                fut = db.get_next_to_send();
                return Ok(Async::Ready(Some(data)));
            }
            val => return val,
        }
    })
}
