package mailer

import (
	"net/http"
	"net/url"
)

// Status checks the status of the service.
func (c *Client) Status() error {
	client, err := c.client(false)
	if err != nil {
		return err
	}

	rel, err := url.Parse("status")
	if err != nil {
		// This indicates a programming error.
		panic(err)
	}

	resp, err := client.Get(c.baseURL.ResolveReference(rel).String())
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNoContent {
		return ErrInvalidServiceBehavior
	}
	return nil
}
