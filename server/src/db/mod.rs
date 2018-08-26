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

use db::schema::{mailer_lists, mailer_queue, mailer_templates, mailer_unsubscribes};
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

    /// Gets a mailing list's name from its ID.
    pub fn get_mailing_list_name(&self, id: u32) -> impl Future<Item = String, Error = Error> {
        self.async_query(move |conn| {
            mailer_lists::table
                .filter(mailer_lists::id.eq(id))
                .select(mailer_lists::name)
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
                let o = mailer_queue::table
                    .inner_join(mailer_templates::table)
                    .inner_join(
                        mailer_lists::table
                            .on(mailer_templates::mailing_list_id.eq(mailer_lists::id)),
                    )
                    .left_join(
                        mailer_unsubscribes::table
                            .on(mailer_queue::email.eq(mailer_unsubscribes::email)),
                    )
                    .filter(
                        diesel::dsl::not(
                            mailer_unsubscribes::email
                                .is_not_null()
                                .and(mailer_unsubscribes::mailing_list_id.eq(mailer_lists::id)),
                        ).and(mailer_queue::send_started.eq(false)),
                    )
                    .select((
                        mailer_queue::id,
                        mailer_templates::mailing_list_id,
                        mailer_queue::template_id,
                        mailer_queue::email,
                        mailer_queue::subject,
                        mailer_queue::data,
                    ))
                    .first(conn)
                    .optional()?;

                if let Some(data) = o {
                    let (id, _, _, _, _, _) = data;
                    diesel::update(mailer_queue::table.filter(mailer_queue::id.eq(id)))
                        .set(mailer_queue::send_started.eq(true))
                        .execute(conn)
                        .map(|_| Some(data))
                } else {
                    Ok(None)
                }
            })
        })
    }

    /// Gets the raw text of a template.
    fn get_template(
        &self,
        mailing_list_id: u32,
        name: String,
    ) -> impl Future<Item = TemplateContents, Error = Error> {
        self.async_query(move |conn| {
            mailer_templates::table
                .filter(mailer_templates::mailing_list_id.eq(mailing_list_id))
                .filter(mailer_templates::name.eq(&name))
                .select((mailer_templates::contents, mailer_templates::markdown))
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
            mailer_lists::table
                .select((mailer_lists::id, mailer_lists::name))
                .load(conn)
        })
    }

    /// Returns a list of template names for the given mailing list.
    pub fn list_templates(
        &self,
        mailing_list_id: u32,
    ) -> impl Future<Item = Vec<String>, Error = Error> {
        self.async_query(move |conn| {
            mailer_templates::table
                .filter(mailer_templates::mailing_list_id.eq(mailing_list_id))
                .select(mailer_templates::name)
                .load(conn)
        })
    }

    /// Loads a template recursively, returning a Tera instance with the required mailer_templates, as
    /// well as the template's name.
    pub fn load_template(
        &self,
        id: u32,
    ) -> impl Future<Item = impl Fn(Context) -> Result<String>, Error = Error> {
        // This can be made a lot more efficient when https://github.com/Keats/tera/issues/322 is
        // resolved. There also may be a more efficient way to write the query (to do one instead
        // of two), but that's probably small potatoes.
        self.async_query(move |conn| -> Result<_> {
            let (mailing_list_id, name) = mailer_templates::table
                .filter(mailer_templates::id.eq(id))
                .select((mailer_templates::mailing_list_id, mailer_templates::name))
                .first::<(u32, String)>(conn)?;
            let mailer_templates = mailer_templates::table
                .filter(mailer_templates::mailing_list_id.eq(mailing_list_id))
                .select((
                    mailer_templates::name,
                    mailer_templates::contents,
                    mailer_templates::markdown,
                ))
                .load::<(String, String, bool)>(conn)?;

            let mut tera = Tera::default();
            for (name, contents, markdown) in mailer_templates {
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
            diesel::insert_into(mailer_lists::table)
                .values(mailer_lists::name.eq(&name))
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
                    mailer_templates::table
                        .filter(mailer_templates::mailing_list_id.eq(mailing_list_id))
                        .filter(mailer_templates::name.eq(&name)),
                )).get_result(conn)?;
                if already_exists {
                    return Err(Error::from(ErrorKind::TemplateExists(name.clone())));
                }

                diesel::insert_into(mailer_templates::table)
                    .values((
                        mailer_templates::mailing_list_id.eq(mailing_list_id),
                        mailer_templates::name.eq(&name),
                        mailer_templates::contents.eq(""),
                    ))
                    .execute(conn)?;
                Ok(())
            })
        })
    }

    /// Marks the sending of an email (by ID) as finished.
    pub fn set_email_done(&self, id: u32) -> impl Future<Item = (), Error = Error> {
        self.async_query(move |conn| {
            diesel::update(mailer_queue::table.filter(mailer_queue::id.eq(id)))
                .filter(mailer_queue::send_started.eq(true))
                .set(mailer_queue::send_done.eq(true))
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
            let target = mailer_templates::table
                .filter(mailer_templates::mailing_list_id.eq(mailing_list_id))
                .filter(mailer_templates::name.eq(&name));
            let (contents, markdown) = match contents {
                TemplateContents::Html(ref s) => (s, false),
                TemplateContents::Markdown(ref s) => (s, true),
            };
            diesel::update(target)
                .set((
                    mailer_templates::contents.eq(contents),
                    mailer_templates::markdown.eq(markdown),
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
            diesel::insert_into(mailer_unsubscribes::table)
                .values((
                    mailer_unsubscribes::email.eq(&email),
                    mailer_unsubscribes::mailing_list_id.eq(mailing_list_id),
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
