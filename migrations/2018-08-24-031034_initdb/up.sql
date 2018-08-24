-- As always, indexes are left as a challenge for the reader -- if it looks like this is slow
-- enough to warrant the time/effort investment, make a migration for them.

CREATE TABLE mailing_lists
	( id   INTEGER UNSIGNED AUTO_INCREMENT PRIMARY KEY
	, name VARCHAR(128) NOT NULL UNIQUE
	);
CREATE TABLE templates
	( id              INTEGER UNSIGNED AUTO_INCREMENT PRIMARY KEY
	, mailing_list_id INTEGER UNSIGNED NOT NULL
	, name            VARCHAR(128) NOT NULL
	, template        LONGTEXT NOT NULL
	, FOREIGN KEY (mailing_list_id) REFERENCES mailing_lists(id)
	);
CREATE TABLE mail_to_send
	( id          INTEGER UNSIGNED AUTO_INCREMENT PRIMARY KEY
	, template_id INTEGER UNSIGNED NOT NULL
	, data   LONGTEXT NOT NULL -- This is JSON, but Maria's JSON type is just an alias for LONGTEXT
		-- (and only very recently supported). Note that even on real MySQL, the JSON type may not
		-- be warranted: the main advantage is that JSON functions are faster on MySQL, but we
		-- don't actually use these.
	, email       VARCHAR(128) NOT NULL
	, created     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
	, sent        TIMESTAMP
	, FOREIGN KEY (template_id) REFERENCES templates(id)
	);
CREATE TABLE mail_unsubscribes
	( id              INTEGER UNSIGNED AUTO_INCREMENT PRIMARY KEY
	, email           VARCHAR(128) NOT NULL
	, mailing_list_id INTEGER UNSIGNED NOT NULL
	, FOREIGN KEY (mailing_list_id) REFERENCES mailing_lists(id)
	);
