use futures::{future::ok, prelude::*, stream::poll_fn};

use {Error, Mailer, DB};

/// Sweeps all unsent emails from the database (by sending them).
pub fn sweep(db: DB, mailer: Mailer) -> impl Future<Item = (), Error = Error> {
    get_all_unsent(db.clone())
        .and_then(move |(id, template_id, email, data)| {
            let db2 = db.clone();
            let mailer2 = mailer.clone();

            let subject = "TODO".to_string(); // TODO

            db.load_template(template_id)
                .and_then(move |render| render(context! {data: data}))
                .and_then(move |body| mailer2.send_mail(email, subject, body))
                .and_then(move |()| db2.set_email_done(id))
        })
        .for_each(|r| {
            // TODO
            println!("{:?}", r);
            ok(())
        })
}

fn get_all_unsent(db: DB) -> impl Stream<Item = (u32, u32, String, String), Error = Error> {
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
