mailer
======

The mailer daemon.

Configuration
-------------

Configured via either environment variables or a `.env` file. The following environment variables are used:

```
# Required
BASE_URL="https://mail.acm.umn.edu/" # Base URL for unsub links and template examples
MAILER_DATABASE_URL="mysql://root:password@localhost/mailer" # MySQL database URL
SMTP_FROM="example@gmail.com" # SMTP From header
SMTP_PASS="hunter2" # SMTP password
SMTP_USER="example@gmail.com" # SMTP username

# Optional
AUTH_SERVER="" # If non-empty, the auth server to use
HOST="::" # IP to bind to
PORT=8000 # Port to serve unsub links and template examples on
SMTP_ADDR="smtp.gmail.com" # SMTP server hostname
SMTP_REPLY_TO="example@gmail.com" # defaults to SMTP_FROM
SYSLOG_SERVER="" # If non-empty, the syslog server to send logs to
```

URL Structure
-------------

### GET `/unsub?email=example@gmail.com&mailing_list=1`

Serves a form asking the user to confirm that they want to be removed from the list.

### POST `/unsub`

The body should contain the same `email` and `mailing_list` parameters as above. Adds a row to the `mail_unsubscribes` table, preventing email form being sent to that address from the given mailing list.

### GET `/template/1`

Requires a cookie granting admin privileges. Renders with the data in the query string.

### POST `/template/1`

Requires a cookie granting admin privileges. Renders with the data in the body.
