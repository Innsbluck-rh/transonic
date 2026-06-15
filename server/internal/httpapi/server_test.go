package httpapi

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/auth"
	"github.com/Innsbluck-rh/transonic/server/internal/config"
	"github.com/Innsbluck-rh/transonic/server/internal/opensubsonic"
	"github.com/Innsbluck-rh/transonic/server/internal/realtime"
	"github.com/Innsbluck-rh/transonic/server/internal/store"
	"github.com/coder/websocket"
	"github.com/coder/websocket/wsjson"
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

func TestWebsocketAcceptsLargePlaybackQueueCommand(t *testing.T) {
	server := testServer(t, opensubsonic.NewClient(nil))
	ctx := context.Background()
	issued, err := server.auth.Issue(ctx, auth.IssueRequest{
		UserKey:           "user",
		DeviceID:          "device-1",
		UpstreamServerURL: "https://upstream.example/rest",
		Username:          "demo",
	})
	if err != nil {
		t.Fatal(err)
	}

	httpServer := httptest.NewServer(server.routes())
	defer httpServer.Close()

	wsURL := "ws" + strings.TrimPrefix(httpServer.URL, "http") + "/v1/ws"
	dialCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()
	conn, _, err := websocket.Dial(dialCtx, wsURL, &websocket.DialOptions{
		HTTPHeader: http.Header{"Authorization": []string{"Bearer " + issued.AccessToken}},
	})
	if err != nil {
		t.Fatal(err)
	}
	defer conn.CloseNow()
	conn.SetReadLimit(connectWebSocketReadLimitBytes)

	queue := largePlaybackQueue(80)
	payload, err := json.Marshal(map[string]any{
		"commandId":    "cmd-large",
		"baseSeq":      0,
		"op":           "setQueue",
		"queue":        queue,
		"currentIndex": 0,
		"autoPlay":     true,
	})
	if err != nil {
		t.Fatal(err)
	}
	command := realtime.Envelope{
		ID:              "cmd-large",
		Type:            realtime.TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         payload,
	}
	rawCommand, err := json.Marshal(command)
	if err != nil {
		t.Fatal(err)
	}
	if len(rawCommand) <= 32768 {
		t.Fatalf("test command should exceed the default websocket read limit, got %d bytes", len(rawCommand))
	}

	writeCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
	err = wsjson.Write(writeCtx, conn, command)
	cancel()
	if err != nil {
		t.Fatal(err)
	}

	readCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()
	for {
		var got realtime.Envelope
		if err := wsjson.Read(readCtx, conn, &got); err != nil {
			t.Fatal(err)
		}
		if got.Type != realtime.TypePlaybackSharedUpdated {
			continue
		}
		var shared struct {
			State struct {
				Queue []json.RawMessage `json:"queue"`
			} `json:"state"`
		}
		if err := json.Unmarshal(got.Payload, &shared); err != nil {
			t.Fatal(err)
		}
		if len(shared.State.Queue) != len(queue) {
			t.Fatalf("expected %d queue entries, got %d", len(queue), len(shared.State.Queue))
		}
		return
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

func largePlaybackQueue(count int) []map[string]any {
	queue := make([]map[string]any, 0, count)
	for i := 1; i <= count; i++ {
		duration := 180
		if i%5 == 0 {
			duration = 1
		}
		queue = append(queue, map[string]any{
			"id":                fmt.Sprintf("song-%04d-0123456789abcdef0123456789abcdef", i),
			"parentId":          "parent-0123456789abcdef",
			"path":              fmt.Sprintf("Artist/Album/%02d - Track title %d.flac", i, i),
			"title":             fmt.Sprintf("Track title %d", i),
			"album":             "Long Album Name With Many Tracks",
			"albumId":           "album-0123456789abcdef0123456789abcdef",
			"artist":            "Artist Name",
			"artistId":          "artist-0123456789abcdef",
			"coverArtId":        "cover-0123456789abcdef",
			"displayCoverArtId": "cover-0123456789abcdef",
			"track":             i,
			"discNumber":        nil,
			"year":              2026,
			"duration":          duration,
			"size":              12345678,
			"contentType":       "audio/flac",
			"suffix":            "flac",
			"bitRate":           900,
			"genre":             nil,
			"created":           nil,
			"starred":           nil,
			"isDirectory":       false,
			"mediaType":         "music",
		})
	}
	return queue
}
