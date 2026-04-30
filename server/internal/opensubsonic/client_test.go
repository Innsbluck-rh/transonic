package opensubsonic

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestNormalizeBaseURL(t *testing.T) {
	got, err := NormalizeBaseURL("https://music.example/base/")
	if err != nil {
		t.Fatal(err)
	}
	if got != "https://music.example/base/rest" {
		t.Fatalf("unexpected normalized URL: %s", got)
	}
}

func TestPasswordAuthUsesTokenAndResolvesUser(t *testing.T) {
	var sawToken bool
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/rest/getOpenSubsonicExtensions.view":
			w.WriteHeader(http.StatusNotFound)
		case "/rest/ping.view":
			if r.URL.Query().Get("u") != "demo" || r.URL.Query().Get("t") == "" || r.URL.Query().Get("s") == "" {
				t.Fatalf("missing token auth params: %s", r.URL.RawQuery)
			}
			if r.URL.Query().Get("p") != "" || r.URL.Query().Get("apiKey") != "" {
				t.Fatalf("password/API key leaked into token auth params: %s", r.URL.RawQuery)
			}
			sawToken = true
			_, _ = w.Write([]byte(`{"subsonic-response":{"status":"ok","version":"1.16.1","type":"Navidrome","serverVersion":"0.54.0","openSubsonic":true}}`))
		default:
			t.Fatalf("unexpected path %s", r.URL.Path)
		}
	}))
	defer server.Close()

	user, err := NewClient(server.Client()).Authenticate(context.Background(), server.URL, AuthInput{
		Kind:     "password",
		Username: "demo",
		Password: "secret",
	})
	if err != nil {
		t.Fatal(err)
	}
	if !sawToken || user.Username != "demo" || user.ServerType != "Navidrome" {
		t.Fatalf("unexpected user: %#v", user)
	}
}

func TestAPIKeyRequiresAdvertisedExtensionAndUsesTokenInfo(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/rest/getOpenSubsonicExtensions.view":
			_, _ = w.Write([]byte(`{"subsonic-response":{"status":"ok","version":"1.16.1","openSubsonic":true,"openSubsonicExtensions":[{"name":"apiKeyAuthentication","versions":[1]}]}}`))
		case "/rest/tokenInfo.view":
			if r.URL.Query().Get("apiKey") != "key" || r.URL.Query().Get("u") != "" || r.URL.Query().Get("t") != "" || r.URL.Query().Get("s") != "" {
				t.Fatalf("invalid api key params: %s", r.URL.RawQuery)
			}
			_, _ = w.Write([]byte(`{"subsonic-response":{"status":"ok","version":"1.16.1","tokenInfo":{"username":"resolved"}}}`))
		case "/rest/ping.view":
			_, _ = w.Write([]byte(`{"subsonic-response":{"status":"ok","version":"1.16.1","openSubsonic":true}}`))
		default:
			t.Fatalf("unexpected path %s", r.URL.Path)
		}
	}))
	defer server.Close()

	user, err := NewClient(server.Client()).Authenticate(context.Background(), server.URL, AuthInput{Kind: "api_key", APIKey: "key"})
	if err != nil {
		t.Fatal(err)
	}
	if user.Username != "resolved" || user.AuthKind != "api_key" {
		t.Fatalf("unexpected user: %#v", user)
	}
}

func TestAPIErrorMapping(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if strings.Contains(r.URL.Path, "getOpenSubsonicExtensions") {
			w.WriteHeader(http.StatusNotFound)
			return
		}
		_, _ = w.Write([]byte(`{"subsonic-response":{"status":"failed","version":"1.16.1","error":{"code":40,"message":"Wrong username or password"}}}`))
	}))
	defer server.Close()

	_, err := NewClient(server.Client()).Authenticate(context.Background(), server.URL, AuthInput{Kind: "password", Username: "demo", Password: "bad"})
	apiErr, ok := err.(*Error)
	if !ok {
		t.Fatalf("expected *Error, got %T", err)
	}
	if apiErr.Kind != ErrorAuth || apiErr.Code != 40 {
		t.Fatalf("unexpected error: %#v", apiErr)
	}
}
