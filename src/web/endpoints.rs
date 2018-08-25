use std::sync::Arc;

use futures::{
    future::{err, Either},
    prelude::*,
};
use tera::Context;
use url::Url;
use warp::http::Response;

use {log_err, Error, ErrorKind, DB};

pub fn template(
    id: u32,
    context: Context,
    auth_server_url: Option<&Url>,
    db: DB,
) -> impl Future<Item = Response<String>, Error = Error> {
    if let Some(auth_server_url) = auth_server_url {
        // TODO: Check auth
        Either::A(
            db.load_template(id)
                .and_then(move |render| render(context).map(Response::new)),
        )
    } else {
        Either::B(err(ErrorKind::NoAuthServer.into()))
    }
}

#[derive(Deserialize)]
pub struct UnsubscribeParams {
    email: String,
}

pub fn unsubscribe_get(
    mailing_list_id: u32,
    params: UnsubscribeParams,
    db: DB,
    render: Arc<impl Fn(&str, Context) -> Response<String>>,
) -> impl Future<Item = Response<String>, Error = Error> {
    db.get_mailing_list_name(mailing_list_id).map(move |name| {
        render(
            "unsubscribe.html",
            context! { email: params.email, name: name },
        )
    })
}

pub fn unsubscribe_post(
    mailing_list_id: u32,
    params: UnsubscribeParams,
    db: DB,
    render: Arc<impl Fn(&str, Context) -> Response<String>>,
) -> impl Future<Item = Response<String>, Error = Error> {
    db.get_mailing_list_name(mailing_list_id)
        .join(db.unsubscribe(params.email.clone(), mailing_list_id))
        .then(move |r| {
            Ok(match r {
                Ok((name, ())) => render(
                    "unsubscribe-ok.html",
                    context! { email: params.email, name: name },
                ),
                Err(e) => {
                    log_err(e.into());
                    render("unsubscribe-err.html", Context::new())
                }
            })
        })
}
