package mailer

import (
	"errors"
	"net/http"
	"net/url"
	"time"
)

var (
	// ErrAuthTokenRequired is issued when a request requires an authentication token, but none was
	// provided to the client at construction.
	ErrAuthTokenRequired = errors.New("Auth token required, but not provided")

	// ErrInvalidServiceBehavior indicates that the server responded in a way not provided for by
	// the client.
	ErrInvalidServiceBehavior = errors.New("Unexpected behavior from mailer service")
)

// Client is a client for the mailer service.
type Client struct {
	// Timeout specifies the time limit for requests.
	Timeout time.Duration

	authCookie string
	baseURL    *url.URL
}

// New creates a new mailer service client. If the authCookie is "", none will be sent, restricting
// the client to the Status and Unsubscribe methods.
func New(baseURL *url.URL, authCookie string) *Client {
	return &Client{
		Timeout:    5 * time.Second,
		authCookie: authCookie,
	}
}

func (c *Client) client(auth bool) (*http.Client, error) {
	var cookies http.CookieJar
	if auth {
		if c.authCookie == "" {
			return nil, ErrAuthTokenRequired
		}
		cookies = authCookieJar(c.authCookie)
	}

	return &http.Client{
		Jar:     cookies,
		Timeout: c.Timeout,
	}, nil
}

type authCookieJar string

func (a authCookieJar) Cookies(u *url.URL) []*http.Cookie {
	return []*http.Cookie{&http.Cookie{
		Name:  "auth",
		Value: string(a),
	}}
}

func (a authCookieJar) SetCookies(u *url.URL, cookies []*http.Cookie) {}
