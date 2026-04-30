package opensubsonic

import (
	"context"
	"crypto/md5"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

const (
	apiVersion = "1.13.0"
	clientName = "transonic-server"
)

type AuthInput struct {
	Kind     string `json:"kind"`
	Username string `json:"username,omitempty"`
	Password string `json:"password,omitempty"`
	APIKey   string `json:"apiKey,omitempty"`
}

type AuthenticatedUser struct {
	NormalizedServerURL string
	Username            string
	AuthKind            string
	APIVersion          string
	ServerType          string
	ServerVersion       string
	OpenSubsonic        bool
	Extensions          []Extension
}

type Extension struct {
	Name     string `json:"name"`
	Versions []int  `json:"versions"`
}

type ErrorKind string

const (
	ErrorAuth            ErrorKind = "auth"
	ErrorUnsupportedAuth ErrorKind = "unsupported_auth"
	ErrorNetwork         ErrorKind = "network"
	ErrorServer          ErrorKind = "server"
	ErrorProtocol        ErrorKind = "protocol"
)

type Error struct {
	Kind    ErrorKind
	Message string
	Code    int
	HelpURL string
}

func (e *Error) Error() string {
	if e.Code != 0 {
		return fmt.Sprintf("%s: API %d: %s", e.Kind, e.Code, e.Message)
	}
	return fmt.Sprintf("%s: %s", e.Kind, e.Message)
}

type Client struct {
	httpClient *http.Client
}

func NewClient(httpClient *http.Client) *Client {
	if httpClient == nil {
		httpClient = &http.Client{Timeout: 15 * time.Second}
	}
	return &Client{httpClient: httpClient}
}

func (c *Client) Authenticate(ctx context.Context, serverURL string, auth AuthInput) (*AuthenticatedUser, error) {
	baseURL, err := NormalizeBaseURL(serverURL)
	if err != nil {
		return nil, &Error{Kind: ErrorNetwork, Message: err.Error()}
	}

	switch auth.Kind {
	case "password":
		username := strings.TrimSpace(auth.Username)
		if username == "" || auth.Password == "" {
			return nil, &Error{Kind: ErrorAuth, Message: "username and password are required"}
		}
		ping, err := c.call(ctx, baseURL, "ping", passwordAuthParams(username, auth.Password))
		if err != nil {
			return nil, err
		}
		extensions, _ := c.getExtensions(ctx, baseURL)
		return userFromEnvelope(baseURL, username, "password", ping, extensions), nil
	case "api_key":
		if auth.APIKey == "" {
			return nil, &Error{Kind: ErrorAuth, Message: "apiKey is required"}
		}
		extensions, err := c.getExtensions(ctx, baseURL)
		if err != nil {
			return nil, err
		}
		if !hasExtension(extensions, "apiKeyAuthentication") {
			return nil, &Error{Kind: ErrorUnsupportedAuth, Message: "upstream server does not advertise apiKeyAuthentication"}
		}
		tokenInfo, err := c.call(ctx, baseURL, "tokenInfo", url.Values{"apiKey": []string{auth.APIKey}})
		if err != nil {
			return nil, err
		}
		if tokenInfo.Payload.TokenInfo == nil || strings.TrimSpace(tokenInfo.Payload.TokenInfo.Username) == "" {
			return nil, &Error{Kind: ErrorProtocol, Message: "tokenInfo response did not include a username"}
		}
		ping, err := c.call(ctx, baseURL, "ping", url.Values{"apiKey": []string{auth.APIKey}})
		if err != nil {
			return nil, err
		}
		return userFromEnvelope(baseURL, tokenInfo.Payload.TokenInfo.Username, "api_key", ping, extensions), nil
	default:
		return nil, &Error{Kind: ErrorUnsupportedAuth, Message: "auth.kind must be password or api_key"}
	}
}

func NormalizeBaseURL(serverURL string) (string, error) {
	trimmed := strings.TrimSpace(serverURL)
	if trimmed == "" {
		return "", errors.New("server URL is required")
	}
	parsed, err := url.Parse(trimmed)
	if err != nil {
		return "", errors.New("enter a valid server URL starting with http:// or https://")
	}
	if parsed.Scheme != "http" && parsed.Scheme != "https" {
		return "", errors.New("server URL must start with http:// or https://")
	}
	if parsed.Host == "" {
		return "", errors.New("server URL must include a host name")
	}
	parsed.User = nil
	parsed.RawQuery = ""
	parsed.Fragment = ""
	path := strings.TrimRight(parsed.EscapedPath(), "/")
	if path == "" {
		path = "/rest"
	} else if !strings.HasSuffix(path, "/rest") {
		path += "/rest"
	}
	parsed.Path = path
	return parsed.String(), nil
}

func UserKey(normalizedServerURL, username string) string {
	sum := sha256.Sum256([]byte(strings.ToLower(normalizedServerURL) + "\x00" + strings.ToLower(strings.TrimSpace(username))))
	return hex.EncodeToString(sum[:])
}

func passwordAuthParams(username, password string) url.Values {
	salt := randomSalt()
	return url.Values{
		"u": []string{strings.TrimSpace(username)},
		"t": []string{token(password, salt)},
		"s": []string{salt},
	}
}

func token(password, salt string) string {
	sum := md5.Sum([]byte(password + salt))
	return hex.EncodeToString(sum[:])
}

func randomSalt() string {
	var b [12]byte
	if _, err := rand.Read(b[:]); err != nil {
		return fmt.Sprintf("%d", time.Now().UnixNano())
	}
	return hex.EncodeToString(b[:])
}

func (c *Client) getExtensions(ctx context.Context, baseURL string) ([]Extension, error) {
	envelope, err := c.call(ctx, baseURL, "getOpenSubsonicExtensions", nil)
	if err != nil {
		var apiErr *Error
		if errors.As(err, &apiErr) && apiErr.Kind == ErrorServer && apiErr.Code == 0 {
			return nil, nil
		}
		return nil, err
	}
	return envelope.Payload.OpenSubsonicExtensions, nil
}

func (c *Client) call(ctx context.Context, baseURL, endpoint string, params url.Values) (*Envelope, error) {
	endpointURL, err := url.Parse(baseURL)
	if err != nil {
		return nil, &Error{Kind: ErrorNetwork, Message: err.Error()}
	}
	endpointURL.Path = strings.TrimRight(endpointURL.Path, "/") + "/" + endpoint + ".view"
	q := endpointURL.Query()
	for key, values := range params {
		for _, value := range values {
			q.Add(key, value)
		}
	}
	q.Set("v", apiVersion)
	q.Set("c", clientName)
	q.Set("f", "json")
	endpointURL.RawQuery = q.Encode()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpointURL.String(), nil)
	if err != nil {
		return nil, &Error{Kind: ErrorNetwork, Message: err.Error()}
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, &Error{Kind: ErrorNetwork, Message: err.Error()}
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(io.LimitReader(resp.Body, 2*1024*1024))
	if err != nil {
		return nil, &Error{Kind: ErrorNetwork, Message: err.Error()}
	}

	if resp.StatusCode == http.StatusUnauthorized || resp.StatusCode == http.StatusForbidden {
		return nil, &Error{Kind: ErrorAuth, Message: fmt.Sprintf("upstream returned HTTP %d", resp.StatusCode)}
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, &Error{Kind: ErrorServer, Message: fmt.Sprintf("upstream returned HTTP %d", resp.StatusCode)}
	}

	var wrapped struct {
		SubsonicResponse Envelope `json:"subsonic-response"`
	}
	if err := json.Unmarshal(body, &wrapped); err != nil {
		return nil, &Error{Kind: ErrorProtocol, Message: "upstream response was not valid OpenSubsonic JSON"}
	}
	envelope := &wrapped.SubsonicResponse
	if envelope.Status == "failed" || envelope.Error != nil {
		return nil, mapAPIError(envelope.Error)
	}
	if envelope.Status != "ok" {
		return nil, &Error{Kind: ErrorProtocol, Message: "upstream response status was not ok"}
	}
	return envelope, nil
}

type Envelope struct {
	Status        string `json:"status"`
	APIVersion    string `json:"version"`
	ServerType    string `json:"type,omitempty"`
	ServerVersion string `json:"serverVersion,omitempty"`
	OpenSubsonic  *bool  `json:"openSubsonic,omitempty"`
	Error         *APIError
	Payload
}

type Payload struct {
	OpenSubsonicExtensions []Extension `json:"openSubsonicExtensions,omitempty"`
	TokenInfo              *TokenInfo  `json:"tokenInfo,omitempty"`
}

type TokenInfo struct {
	Username string `json:"username"`
}

type APIError struct {
	Code    int    `json:"code"`
	Message string `json:"message,omitempty"`
	HelpURL string `json:"helpUrl,omitempty"`
}

func mapAPIError(apiErr *APIError) error {
	if apiErr == nil {
		return &Error{Kind: ErrorServer, Message: "upstream returned failed status"}
	}
	switch apiErr.Code {
	case 40, 44:
		return &Error{Kind: ErrorAuth, Code: apiErr.Code, Message: messageOrDefault(apiErr.Message, "upstream rejected credentials"), HelpURL: apiErr.HelpURL}
	case 41, 42:
		return &Error{Kind: ErrorUnsupportedAuth, Code: apiErr.Code, Message: messageOrDefault(apiErr.Message, "upstream does not support selected authentication"), HelpURL: apiErr.HelpURL}
	case 43:
		return &Error{Kind: ErrorProtocol, Code: apiErr.Code, Message: messageOrDefault(apiErr.Message, "upstream reported conflicting authentication parameters"), HelpURL: apiErr.HelpURL}
	default:
		return &Error{Kind: ErrorServer, Code: apiErr.Code, Message: messageOrDefault(apiErr.Message, "upstream rejected request"), HelpURL: apiErr.HelpURL}
	}
}

func messageOrDefault(message, fallback string) string {
	if message == "" {
		return fallback
	}
	return message
}

func userFromEnvelope(baseURL, username, authKind string, envelope *Envelope, extensions []Extension) *AuthenticatedUser {
	openSubsonic := false
	if envelope.OpenSubsonic != nil {
		openSubsonic = *envelope.OpenSubsonic
	}
	if hasExtension(extensions, "apiKeyAuthentication") || len(extensions) > 0 {
		openSubsonic = true
	}
	return &AuthenticatedUser{
		NormalizedServerURL: baseURL,
		Username:            strings.TrimSpace(username),
		AuthKind:            authKind,
		APIVersion:          envelope.APIVersion,
		ServerType:          envelope.ServerType,
		ServerVersion:       envelope.ServerVersion,
		OpenSubsonic:        openSubsonic,
		Extensions:          extensions,
	}
}

func hasExtension(extensions []Extension, name string) bool {
	for _, extension := range extensions {
		if extension.Name == name {
			return true
		}
	}
	return false
}
