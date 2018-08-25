//! The web-serving parts.

mod endpoints;

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::prelude::*;
use serde_json::Value;
use tera::{self, Context, Tera};
use url::Url;
use warp::{
    self,
    filters::BoxedFilter,
    http::{
        header::{HeaderValue, CONTENT_TYPE},
        status::StatusCode,
        Response,
    },
    reject, Filter,
};

use {log_err, web::endpoints::*, DB};

/// Returns all the routes.
pub fn routes(
    db: DB,
    auth_server_url: Option<Url>,
    base_url: Arc<Url>,
) -> BoxedFilter<(impl warp::Reply,)> {
    let mut tera = Tera::default();
    tera.register_global_function(
        "relative_url",
        Box::new(move |args| {
            let s = try_get_value!("relative_url", "path", String, args["path"]);
            let url = base_url.join(&s).map_err(|e| e.to_string())?;
            Ok(tera::to_value(&url.to_string()).unwrap())
        }),
    );
    tera.add_raw_templates(vec![
        ("base.html", include_str!("base.html")),
        ("index.html", include_str!("index.html")),
        ("unsubscribe.html", include_str!("unsubscribe.html")),
        ("unsubscribe-ok.html", include_str!("unsubscribe-ok.html")),
        ("unsubscribe-err.html", include_str!("unsubscribe-err.html")),
    ]).expect("Template error");

    let render = Arc::new(move |name: &str, context: Context| -> Response<String> {
        match tera.render(name, &context) {
            Ok(html) => {
                let mut res = Response::new(html);
                res.headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
                res
            }
            Err(e) => {
                let mut s = e
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join("\n");
                error!("{}", s);

                let mut res = Response::new(s);
                res.headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
                *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                res
            }
        }
    });
    let render2 = render.clone();
    let render3 = render.clone();

    let db2 = db.clone();
    let db3 = db.clone();
    let db4 = db.clone();

    warp::index()
        .map(move || render("index.html", Context::new()))
        .or(path!("main.css").and(warp::index()).map(|| {
            let mut res = Response::new(include_str!("main.css").to_string());
            res.headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("text/css"));
            res
        }))
        .or(path!("send")
            .and(warp::index())
            .and(warp::post2())
            .and(warp::body::form())
            .and_then(move |params| {
                send(params, db.clone()).map_err(|e| {
                    log_err(e.into());
                    reject::server_error()
                })
            }))
        .or(path!("status")
            .and(warp::index())
            .and(warp::get2())
            .map(|| {
                let mut res = Response::new("".to_string());
                *res.status_mut() = StatusCode::NO_CONTENT;
                res
            }))
        .or(path!("template" / u32)
            .and(warp::index())
            .and(
                warp::get2()
                    .and(warp::query::<BTreeMap<String, Value>>())
                    .or(warp::post2().and(warp::body::form::<BTreeMap<String, Value>>()))
                    .unify(),
            )
            .and_then(move |template_id: u32, values: BTreeMap<String, Value>| {
                let mut context = Context::new();
                for (k, v) in values {
                    context.add(&k, &v);
                }
                template(template_id, context, auth_server_url.as_ref(), db2.clone()).map_err(|e| {
                    log_err(e.into());
                    reject::server_error()
                })
            }))
        .or(path!("unsubscribe" / u32)
            .and(warp::index())
            .and(warp::get2())
            .and(warp::query())
            .and_then(move |mailing_list_id, params| {
                unsubscribe_get(mailing_list_id, params, db3.clone(), render2.clone()).map_err(
                    |e| {
                        log_err(e.into());
                        reject::server_error()
                    },
                )
            }))
        .or(path!("unsubscribe" / u32)
            .and(warp::index())
            .and(warp::post2())
            .and(warp::body::form())
            .and_then(move |mailing_list_id, params| {
                unsubscribe_post(mailing_list_id, params, db4.clone(), render3.clone()).map_err(
                    |e| {
                        log_err(e.into());
                        reject::server_error()
                    },
                )
            }))
        .boxed()
}
