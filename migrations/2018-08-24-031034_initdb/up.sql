CREATE TABLE mailing_lists
	( id   INTEGER UNSIGNED AUTO_INCREMENT PRIMARY KEY
	, name VARCHAR(128) NOT NULL
	);
CREATE TABLE templates
	( id              INTEGER UNSIGNED AUTO_INCREMENT PRIMARY KEY
	, mailing_list_id INTEGER UNSIGNED NOT NULL
	, name            VARCHAR(128) NOT NULL
	, template        TEXT NOT NULL
	, FOREIGN KEY (mailing_list_id) REFERENCES mailing_lists(id)
	);
CREATE TABLE mail_to_send
	( id          INTEGER UNSIGNED AUTO_INCREMENT PRIMARY KEY
	, template_id INTEGER UNSIGNED NOT NULL
	, email       VARCHAR(128) NOT NULL
	, created     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
	, sent        DATETIME
	, FOREIGN KEY (template_id) REFERENCES templates(id)
	);
CREATE TABLE mail_unsubscribes
	( id              INTEGER UNSIGNED AUTO_INCREMENT PRIMARY KEY
	, email           VARCHAR(128) NOT NULL
	, mailing_list_id INTEGER UNSIGNED NOT NULL
	, FOREIGN KEY (mailing_list_id) REFERENCES mailing_lists(id)
	);
