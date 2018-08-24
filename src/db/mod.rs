//! The database and related types.

mod schema;

use std::sync::Arc;

use diesel::{
    self,
    prelude::*,
    r2d2::{ConnectionManager, Pool},
};
use futures::{
    future::{err, poll_fn, Either},
    prelude::*,
};
use pulldown_cmark::{self, html::push_html};
use tera::{Context, Tera};
use tokio_threadpool::blocking;

use db::schema::{mail_to_send, mail_unsubscribes, mailing_lists, templates};
use {Error, ErrorKind, Result};

/// An HTML or Markdown document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplateContents {
    /// An HTML template.
    Html(String),

    /// A Markdown template.
    Markdown(String),
}

/// A pool of connections to the database.
#[derive(Clone)]
pub struct DB {
    pool: Arc<Pool<ConnectionManager<MysqlConnection>>>,
}

impl DB {
    /// Connects to the database with the given number of connections.
    pub fn connect(database_url: &str) -> Result<DB> {
        let pool = Arc::new(Pool::new(ConnectionManager::new(database_url))?);
        Ok(DB { pool })
    }

    /// Gets a mailing list's ID from its name.
    pub fn get_mailing_list_id(&self, name: String) -> impl Future<Item = u32, Error = Error> {
        self.async_query(move |conn| {
            mailing_lists::table
                .filter(mailing_lists::name.eq(&name))
                .select(mailing_lists::id)
                .first(conn)
        })
    }

    /// Gets the next mail item to be sent. The fields are, in order,
    /// `(id, mailing_list_id, template_id, to_addr, subject, data)`.
    pub fn get_next_to_send(
        &self,
    ) -> impl Future<Item = Option<(u32, u32, u32, String, String, String)>, Error = Error> {
        self.async_query(move |conn| {
            conn.transaction(|| {
                let o = mail_to_send::table
                    .inner_join(templates::table)
                    .inner_join(
                        mailing_lists::table.on(templates::mailing_list_id.eq(mailing_lists::id)),
                    )
                    .left_join(
                        mail_unsubscribes::table
                            .on(mail_to_send::email.eq(mail_unsubscribes::email)),
                    )
                    .filter(
                        diesel::dsl::not(
                            mail_unsubscribes::email
                                .is_not_null()
                                .and(mail_unsubscribes::mailing_list_id.eq(mailing_lists::id)),
                        ).and(mail_to_send::send_started.eq(false)),
                    )
                    .select((
                        mail_to_send::id,
                        templates::mailing_list_id,
                        mail_to_send::template_id,
                        mail_to_send::email,
                        mail_to_send::subject,
                        mail_to_send::data,
                    ))
                    .first(conn)
                    .optional()?;

                if let Some(data) = o {
                    let (id, _, _, _, _, _) = data;
                    diesel::update(mail_to_send::table.filter(mail_to_send::id.eq(id)))
                        .set(mail_to_send::send_started.eq(true))
                        .execute(conn)
                        .map(|_| Some(data))
                } else {
                    Ok(None)
                }
            })
        })
    }

    /// Gets the raw text of a template.
    pub fn get_template(
        &self,
        mailing_list_id: u32,
        name: String,
    ) -> impl Future<Item = TemplateContents, Error = Error> {
        self.async_query(move |conn| {
            templates::table
                .filter(templates::mailing_list_id.eq(mailing_list_id))
                .filter(templates::name.eq(&name))
                .select((templates::contents, templates::markdown))
                .first::<(String, bool)>(conn)
                .map(|(contents, markdown)| {
                    if markdown {
                        TemplateContents::Markdown(contents)
                    } else {
                        TemplateContents::Html(contents)
                    }
                })
        })
    }

    /// Returns a list of mailing lists.
    pub fn list_mailing_lists(&self) -> impl Future<Item = Vec<(u32, String)>, Error = Error> {
        self.async_query(move |conn| {
            mailing_lists::table
                .select((mailing_lists::id, mailing_lists::name))
                .load(conn)
        })
    }

    /// Returns a list of template names for the given mailing list.
    pub fn list_templates(
        &self,
        mailing_list_id: u32,
    ) -> impl Future<Item = Vec<String>, Error = Error> {
        self.async_query(move |conn| {
            templates::table
                .filter(templates::mailing_list_id.eq(mailing_list_id))
                .select(templates::name)
                .load(conn)
        })
    }

    /// Loads a template recursively, returning a Tera instance with the required templates, as
    /// well as the template's name.
    pub fn load_template(
        &self,
        id: u32,
    ) -> impl Future<Item = impl Fn(Context) -> Result<String>, Error = Error> {
        // This can be made a lot more efficient when https://github.com/Keats/tera/issues/322 is
        // resolved. There also may be a more efficient way to write the query (to do one instead
        // of two), but that's probably small potatoes.
        self.async_query(move |conn| -> Result<_> {
            let (mailing_list_id, name) = templates::table
                .filter(templates::id.eq(id))
                .select((templates::mailing_list_id, templates::name))
                .first::<(u32, String)>(conn)?;
            let templates = templates::table
                .filter(templates::mailing_list_id.eq(mailing_list_id))
                .select((templates::name, templates::contents, templates::markdown))
                .load::<(String, String, bool)>(conn)?;

            let mut tera = Tera::default();
            for (name, contents, markdown) in templates {
                let contents = if markdown {
                    let mut html = String::new();
                    push_html(&mut html, pulldown_cmark::Parser::new(&contents));
                    html
                } else {
                    contents
                };
                tera.add_raw_template(&name, &contents)?;
            }
            tera.build_inheritance_chains()?;

            Ok(move |data| tera.render(&name, &data).map_err(Error::from))
        })
    }

    /// Creates a new mailing list with the given name.
    pub fn new_mailing_list(&self, name: String) -> impl Future<Item = (), Error = Error> {
        self.async_query(move |conn| {
            diesel::insert_into(mailing_lists::table)
                .values(mailing_lists::name.eq(&name))
                .execute(conn)
                .map(|_| ())
        })
    }

    /// Creates a new template with the given name to the mailing list with the given ID.
    pub fn new_template(
        &self,
        mailing_list_id: u32,
        name: String,
    ) -> impl Future<Item = (), Error = Error> {
        self.async_query(move |conn| {
            conn.transaction(|| {
                let already_exists = diesel::select(diesel::dsl::exists(
                    templates::table
                        .filter(templates::mailing_list_id.eq(mailing_list_id))
                        .filter(templates::name.eq(&name)),
                )).get_result(conn)?;
                if already_exists {
                    return Err(Error::from(ErrorKind::TemplateExists(name.clone())));
                }

                diesel::insert_into(templates::table)
                    .values((
                        templates::mailing_list_id.eq(mailing_list_id),
                        templates::name.eq(&name),
                        templates::contents.eq(""),
                    ))
                    .execute(conn)?;
                Ok(())
            })
        })
    }

    /// Marks the sending of an email (by ID) as finished.
    pub fn set_email_done(&self, id: u32) -> impl Future<Item = (), Error = Error> {
        self.async_query(move |conn| {
            diesel::update(mail_to_send::table.filter(mail_to_send::id.eq(id)))
                .filter(mail_to_send::send_started.eq(true))
                .set(mail_to_send::send_done.eq(true))
                .execute(conn)
                .map(|_| ())
        })
    }

    /// Sets the contents of the template with the given name.
    pub fn set_template(
        &self,
        mailing_list_id: u32,
        name: String,
        contents: TemplateContents,
    ) -> impl Future<Item = (), Error = Error> {
        self.async_query(move |conn| {
            let target = templates::table
                .filter(templates::mailing_list_id.eq(mailing_list_id))
                .filter(templates::name.eq(&name));
            let (contents, markdown) = match contents {
                TemplateContents::Html(ref s) => (s, false),
                TemplateContents::Markdown(ref s) => (s, true),
            };
            diesel::update(target)
                .set((
                    templates::contents.eq(contents),
                    templates::markdown.eq(markdown),
                ))
                .execute(conn)
                .map(|_| ())
        })
    }

    /// Marks a user as having unsubscribed from the given mailing list.
    pub fn unsubscribe(
        &self,
        email: String,
        mailing_list_id: u32,
    ) -> impl Future<Item = (), Error = Error> {
        self.async_query(move |conn| {
            diesel::insert_into(mail_unsubscribes::table)
                .values((
                    mail_unsubscribes::email.eq(&email),
                    mail_unsubscribes::mailing_list_id.eq(mailing_list_id),
                ))
                .execute(conn)
                .map(|_| ())
        })
    }

    fn async_query<E, F, T>(&self, func: F) -> impl Future<Item = T, Error = Error>
    where
        E: Into<Error>,
        F: Fn(&MysqlConnection) -> ::std::result::Result<T, E>,
    {
        match self.pool.get() {
            Ok(conn) => Either::A(
                poll_fn(move || {
                    blocking(|| func(&*conn).map_err(|e| e.into())).map_err(|_| {
                        panic!("Database queries must be run inside a Tokio thread pool!")
                    })
                }).and_then(|r| r),
            ),
            Err(e) => Either::B(err(e.into())),
        }
    }
}
