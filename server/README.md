mailer
======

The mailer daemon.

Configuration
-------------

Configured via either environment variables or a `.env` file. The following environment variables are used:

```
# Required
AUTH_SERVER="https://auth.acm.umn.edu" # The URL of the auth server to use
AUTH_TOKEN="..." # This service's authentication token
BASE_URL="https://mail.acm.umn.edu" # Base URL for unsub links and template examples
DATABASE_URL="mysql://root:password@localhost/acm" # MySQL database URL
SMTP_FROM="example@gmail.com" # SMTP From header
SMTP_PASS="hunter2" # SMTP password
SMTP_USER="example@gmail.com" # SMTP username

# Optional
HOST="::" # IP to bind to
PORT=8000 # Port to serve unsub links and template examples on
SMTP_ADDR="smtp.gmail.com" # SMTP server hostname
SMTP_REPLY_TO="example@gmail.com" # defaults to SMTP_FROM
SYSLOG_SERVER="" # If non-empty, the syslog server to send logs to
```

URL Structure
-------------

### GET `/unsubscribe/<list-id>?email=example@gmail.com`

Serves a form asking the user to confirm that they want to be removed from the list.

### POST `/unsubscribe/<list-id>`

Adds a row to the `mail_unsubscribes` table, preventing email form being sent to that address from the given mailing list. A request `Content-Type` of `application/x-www-form-urlencoded` is required. The body should contain the same `email` parameter as above.

### GET `/template/<template-id>`

Requires an authentication token granting admin privileges. Renders the template with the data in the query string.

### POST `/template/<template-id>`

Requires an authentication token granting admin privileges. A request `Content-Type` of `application/x-www-form-urlencoded` is required. Renders the template with the data in the body.

### GET `/status`

Always responds with an HTTP 204.

### POST `/send`

Requires a service authentication token. A request `Content-Type` of `application/x-www-form-urlencoded` is required. The body of the request should contain:

-	`mailing_list` -- The name of the mailing list.
-	`template` -- The name of the template.
-	`data` -- A JSON string containing the data to render into the template.
-	`email` -- The email address to send to.
-	`subject` -- The subject line of the email.

If everything is valid, will respond with an HTTP 202.
