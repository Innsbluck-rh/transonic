package httpapi

import (
	"bytes"
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/auth"
	"github.com/Innsbluck-rh/transonic/server/internal/config"
	"github.com/Innsbluck-rh/transonic/server/internal/opensubsonic"
	"github.com/Innsbluck-rh/transonic/server/internal/realtime"
	"github.com/Innsbluck-rh/transonic/server/internal/store"
)

func TestHealthVersionCapabilities(t *testing.T) {
	server := testServer(t, opensubsonic.NewClient(nil))
	for _, path := range []string{"/healthz", "/version", "/v1/capabilities"} {
		req := httptest.NewRequest(http.MethodGet, path, nil)
		res := httptest.NewRecorder()
		server.routes().ServeHTTP(res, req)
		if res.Code != http.StatusOK {
			t.Fatalf("%s returned %d", path, res.Code)
		}
	}
}

func TestLoginStoresDeviceAndReturnsToken(t *testing.T) {
	upstream := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/rest/getOpenSubsonicExtensions.view":
			w.WriteHeader(http.StatusNotFound)
		case "/rest/ping.view":
			_, _ = w.Write([]byte(`{"subsonic-response":{"status":"ok","version":"1.16.1"}}`))
		default:
			t.Fatalf("unexpected upstream path %s", r.URL.Path)
		}
	}))
	defer upstream.Close()
	server := testServer(t, opensubsonic.NewClient(upstream.Client()))

	body := []byte(`{
  "upstreamServerUrl": "` + upstream.URL + `",
  "auth": {"kind":"password","username":"demo","password":"secret"},
  "device": {"deviceId":"device-1","displayName":"Desktop","platform":"windows","appVersion":"0.1.0"}
}`)
	req := httptest.NewRequest(http.MethodPost, "/v1/auth/login", bytes.NewReader(body))
	res := httptest.NewRecorder()
	server.routes().ServeHTTP(res, req)
	if res.Code != http.StatusOK {
		t.Fatalf("login returned %d: %s", res.Code, res.Body.String())
	}
	var response LoginResponse
	if err := json.Unmarshal(res.Body.Bytes(), &response); err != nil {
		t.Fatal(err)
	}
	if response.AccessToken == "" || response.UserKey == "" || response.DeviceID != "device-1" {
		t.Fatalf("unexpected response: %#v", response)
	}
}

func testServer(t *testing.T, upstream *opensubsonic.Client) *Server {
	t.Helper()
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = st.Close() })
	manager, err := auth.NewManager(st, []byte("1234567890123456"), time.Hour)
	if err != nil {
		t.Fatal(err)
	}
	cfg := config.Default()
	cfg.AllowedOrigins = []string{"*"}
	return New(cfg, st, manager, upstream, realtime.NewHub(st, time.Minute), slog.Default())
}
