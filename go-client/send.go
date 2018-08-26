package mailer

import (
	"encoding/json"
	"net/http"
	"net/url"
)

// Send enqueues a mail.
func (c *Client) Send(mailingList, template string, data map[string]interface{}, email string,
	subject string) error {

	client, err := c.client(true)
	if err != nil {
		return err
	}

	rel, err := url.Parse("send")
	if err != nil {
		// This indicates a programming error.
		panic(err)
	}

	dataEnc, err := json.Marshal(data)
	if err != nil {
		return err
	}

	resp, err := client.PostForm(c.baseURL.ResolveReference(rel).String(), url.Values{
		"mailing_list": []string{mailingList},
		"template":     []string{template},
		"data":         []string{string(dataEnc)},
		"email":        []string{email},
		"subject":      []string{subject},
	})
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusAccepted {
		return ErrInvalidServiceBehavior
	}
	return nil
}
