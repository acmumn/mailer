package mailer

import (
	"fmt"
	"net/http"
	"net/url"
)

// Unsubscribe unsubscribes an email address from the given mailing list.
func (c *Client) Unsubscribe(mailingList uint, email string) error {
	client, err := c.client(false)
	if err != nil {
		return err
	}

	rel, err := url.Parse(fmt.Sprintf("unsubscribe/%d", mailingList))
	if err != nil {
		return err
	}

	resp, err := client.PostForm(c.baseURL.ResolveReference(rel).String(), url.Values{
		"email": []string{email},
	})
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return ErrInvalidServiceBehavior
	}
	return nil
}
