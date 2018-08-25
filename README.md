mailer
======

The mailer daemon.

Configuration
-------------

Configured via either environment variables or a `.env` file. The following environment variables are used:

```
# Required
BASE_URL="https://mail.acm.umn.edu" # Base URL for unsub links and template examples
DATABASE_URL="mysql://root:password@localhost/mailer" # MySQL database URL
SMTP_FROM="example@gmail.com" # SMTP From header
SMTP_PASS="hunter2" # SMTP password
SMTP_USER="example@gmail.com" # SMTP username

# Optional
AUTH_SERVER="http://auth" # The URL of the auth server to use
HOST="::" # IP to bind to
PORT=8000 # Port to serve unsub links and template examples on
SMTP_ADDR="smtp.gmail.com" # SMTP server hostname
SMTP_REPLY_TO="example@gmail.com" # defaults to SMTP_FROM
SYSLOG_SERVER="" # If non-empty, the syslog server to send logs to
```

URL Structure
-------------

### GET `/unsubscribe/1?email=example@gmail.com`

Serves a form asking the user to confirm that they want to be removed from the list.

### POST `/unsubscribe/1`

The body should contain the same `email` parameter as above. Adds a row to the `mail_unsubscribes` table, preventing email form being sent to that address from the given mailing list.

### GET `/template/1`

Requires a cookie granting admin privileges and `AUTH_SERVER` to be defined. Renders with the data in the query string.

### POST `/template/1`

Requires a cookie granting admin privileges and `AUTH_SERVER` to be defined. Renders with the data in the body.

### GET `/status`

Always responds with an HTTP 204.

### POST `/send`

Requires a service cookie and `AUTH_SERVER` to be defined. The body should contain:

-	`mailing_list` -- The name of the mailing list.
-	`template` -- The name of the template.
-	`data` -- A JSON string containing the data to render into the template.
-	`email` -- The email address to send to.
-	`subject` -- The subject line of the email.

If everything is valid, will respond with an HTTP 202.
