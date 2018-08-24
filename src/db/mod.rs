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
use tokio_threadpool::blocking;

pub use db::schema::{mail_to_send, mail_unsubscribes, mailing_lists, templates};
use {Error, ErrorKind, Result};

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
                        templates::template.eq(""),
                    ))
                    .execute(conn)?;
                Ok(())
            })
        })
    }

    /// Gets a mailing list's ID from its name.
    pub fn get_mailing_list(&self, name: String) -> impl Future<Item = u32, Error = Error> {
        self.async_query(move |conn| {
            mailing_lists::table
                .filter(mailing_lists::name.eq(&name))
                .select(mailing_lists::id)
                .first(conn)
        })
    }

    /// Gets the next mail item to be sent. The fields are, in order,
    /// `(id, email, data, template_name, template_data)`.
    pub fn get_next_to_send(
        &self,
    ) -> impl Future<Item = Option<(u32, String, String, String, String)>, Error = Error> {
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
                        mail_to_send::email,
                        mail_to_send::data,
                        templates::name,
                        templates::template,
                    ))
                    .first(conn)
                    .optional()?;

                if let Some((id, email, data, template_name, template_data)) = o {
                    diesel::update(mail_to_send::table.filter(mail_to_send::id.eq(id)))
                        .set(mail_to_send::send_started.eq(true))
                        .execute(conn)
                        .map(|_| Some((id, email, data, template_name, template_data)))
                } else {
                    Ok(None)
                }
            })
        })
    }

    /// Gets a template from the given list with the given name from the database.
    pub fn get_template(
        &self,
        mailing_list_id: u32,
        name: String,
    ) -> impl Future<Item = String, Error = Error> {
        self.async_query(move |conn| {
            templates::table
                .filter(templates::mailing_list_id.eq(mailing_list_id))
                .filter(templates::name.eq(&name))
                .select(templates::template)
                .first(conn)
        })
    }

    /// Marks the sending of an email (by ID) as finished.
    pub fn set_email_done(&self, id: u32) -> impl Future<Item = (), Error = Error> {
        self.async_query(move |conn| {
            diesel::update(mail_to_send::table.filter(mail_to_send::id.eq(id)))
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
        contents: String,
    ) -> impl Future<Item = (), Error = Error> {
        self.async_query(move |conn| {
            let target = templates::table
                .filter(templates::mailing_list_id.eq(mailing_list_id))
                .filter(templates::name.eq(&name));
            diesel::update(target)
                .set(templates::template.eq(&contents))
                .execute(conn)
                .map(|_| ())
        })
    }

    /*

    /// Checks whether a login UUID is valid, returning the relevant member ID.
    pub fn check_login_link(&self, uuid: Uuid) -> impl Future<Item = MemberID, Error = Error> {
        self.async_query(move |conn| {
            conn.transaction(|| {
                let (id, member_id, created): (u32, _, _) = jwt_escrow::table
                    .select((jwt_escrow::id, jwt_escrow::member_id, jwt_escrow::created))
                    .filter(jwt_escrow::secret.eq(uuid.as_bytes() as &[u8]))
                    .first(conn)?;
                diesel::delete(jwt_escrow::table)
                    .filter(jwt_escrow::id.eq(id))
                    .execute(conn)?;
                let created = DateTime::<Utc>::from_utc(created, Utc);
                if Utc::now().signed_duration_since(created) > Duration::hours(24) {
                    Err(Error::from(LoginLinkExpired))
                } else {
                    Ok(member_id)
                }
            })
        })
    }

    /// Creates a new login UUID for the member with the given ID, returning it.
    pub fn create_login_link(&self, id: MemberID) -> impl Future<Item = Uuid, Error = Error> {
        let uuid = Uuid::new_v4();
        self.async_query(move |conn| {
            diesel::insert_into(jwt_escrow::table)
                .values((
                    jwt_escrow::member_id.eq(id),
                    jwt_escrow::secret.eq(uuid.as_bytes() as &[u8]),
                ))
                .execute(conn)
                .map(|_| uuid)
        })
    }

    /// Gets a member by ID.
    pub fn get_member(&self, id: MemberID) -> impl Future<Item = Member, Error = Error> {
        self.async_query(move |conn| members::table.filter(members::id.eq(id)).first(conn))
    }

    /// Gets an ID number by X.500.
    pub fn get_id(&self, x500: String) -> impl Future<Item = MemberID, Error = Error> {
        self.async_query(move |conn| {
            members::table
                .filter(members::x500.eq(&x500))
                .select(members::id)
                .first(conn)
        })
    }

    /// Checks if a member is an admin.
    pub fn is_admin(&self, id: MemberID) -> impl Future<Item = bool, Error = Error> {
        self.async_query(move |conn| {
            members::table
                .filter(members::id.eq(id))
                .select(members::admin)
                .first(conn)
        })
    }

    /// Lists all members in the database.
    pub fn list_members(&self) -> impl Future<Item = Vec<Member>, Error = Error> {
        self.async_query(|conn| members::table.load(conn))
    }

    /// Lists all members who are in good standing with regards to payment at the given time.
    pub fn list_paid_members<Tz: TimeZone>(
        &self,
        time: DateTime<Tz>,
    ) -> impl Future<Item = Vec<Member>, Error = Error> {
        let time = time.naive_utc();
        self.async_query(move |conn| {
            payments::table
                .filter(payments::date_from.lt(time))
                .filter(payments::date_to.ge(time))
                .inner_join(members::table)
                .select(members::all_columns)
                .load(conn)
        })
    }

    */

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
