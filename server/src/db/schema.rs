table! {
    mailer_lists (id) {
        id -> Unsigned<Integer>,
        name -> Varchar,
    }
}

table! {
    mailer_queue (id) {
        id -> Unsigned<Integer>,
        template_id -> Unsigned<Integer>,
        data -> Longtext,
        email -> Varchar,
        subject -> Varchar,
        send_started -> Bool,
        send_done -> Bool,
    }
}

table! {
    mailer_templates (id) {
        id -> Unsigned<Integer>,
        mailing_list_id -> Unsigned<Integer>,
        name -> Varchar,
        contents -> Longtext,
        markdown -> Bool,
    }
}

table! {
    mailer_unsubscribes (id) {
        id -> Unsigned<Integer>,
        email -> Varchar,
        mailing_list_id -> Unsigned<Integer>,
    }
}

joinable!(mailer_queue -> mailer_templates (template_id));
joinable!(mailer_templates -> mailer_lists (mailing_list_id));
joinable!(mailer_unsubscribes -> mailer_lists (mailing_list_id));

allow_tables_to_appear_in_same_query!(
    mailer_lists,
    mailer_queue,
    mailer_templates,
    mailer_unsubscribes,
);
