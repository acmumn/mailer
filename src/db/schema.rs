table! {
    mailing_lists (id) {
        id -> Unsigned<Integer>,
        name -> Varchar,
    }
}

table! {
    mail_to_send (id) {
        id -> Unsigned<Integer>,
        template_id -> Unsigned<Integer>,
        mail_data -> Longtext,
        email -> Varchar,
        created -> Timestamp,
        sent -> Timestamp,
    }
}

table! {
    mail_unsubscribes (id) {
        id -> Unsigned<Integer>,
        email -> Varchar,
        mailing_list_id -> Unsigned<Integer>,
    }
}

table! {
    templates (id) {
        id -> Unsigned<Integer>,
        mailing_list_id -> Unsigned<Integer>,
        name -> Varchar,
        template -> Longtext,
    }
}

joinable!(mail_to_send -> templates (template_id));
joinable!(mail_unsubscribes -> mailing_lists (mailing_list_id));
joinable!(templates -> mailing_lists (mailing_list_id));

allow_tables_to_appear_in_same_query!(
    mailing_lists,
    mail_to_send,
    mail_unsubscribes,
    templates,
);