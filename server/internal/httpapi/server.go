package httpapi

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"log/slog"
	"net/http"
	"strings"
	"time"

	"github.com/coder/websocket"
	"github.com/coder/websocket/wsjson"

	"github.com/Innsbluck-rh/transonic/server/internal/auth"
	"github.com/Innsbluck-rh/transonic/server/internal/config"
	"github.com/Innsbluck-rh/transonic/server/internal/opensubsonic"
	"github.com/Innsbluck-rh/transonic/server/internal/realtime"
	"github.com/Innsbluck-rh/transonic/server/internal/store"
	"github.com/Innsbluck-rh/transonic/server/internal/version"
)

type Server struct {
	config     config.Config
	store      *store.Store
	auth       *auth.Manager
	upstream   *opensubsonic.Client
	hub        *realtime.Hub
	logger     *slog.Logger
	httpServer *http.Server
}

type DeviceRequest struct {
	DeviceID    string `json:"deviceId,omitempty"`
	DisplayName string `json:"displayName,omitempty"`
	Platform    string `json:"platform,omitempty"`
	AppVersion  string `json:"appVersion,omitempty"`
}

type LoginRequest struct {
	UpstreamServerURL string                 `json:"upstreamServerUrl"`
	Auth              opensubsonic.AuthInput `json:"auth"`
	Device            DeviceRequest          `json:"device"`
}

type LoginResponse struct {
	AccessToken string    `json:"accessToken"`
	ExpiresAt   time.Time `json:"expiresAt"`
	UserKey     string    `json:"userKey"`
	DeviceID    string    `json:"deviceId"`
}

type ErrorResponse struct {
	Error   string `json:"error"`
	Code    int    `json:"code,omitempty"`
	HelpURL string `json:"helpUrl,omitempty"`
}

func New(cfg config.Config, st *store.Store, authManager *auth.Manager, upstream *opensubsonic.Client, hub *realtime.Hub, logger *slog.Logger) *Server {
	if logger == nil {
		logger = slog.Default()
	}
	server := &Server{
		config:   cfg,
		store:    st,
		auth:     authManager,
		upstream: upstream,
		hub:      hub,
		logger:   logger,
	}
	server.httpServer = &http.Server{
		Addr:              cfg.ListenAddress,
		Handler:           server.routes(),
		ReadHeaderTimeout: 10 * time.Second,
	}
	return server
}

func (s *Server) ListenAndServe() error {
	if s.config.TLSCertFile != "" {
		return s.httpServer.ListenAndServeTLS(s.config.TLSCertFile, s.config.TLSKeyFile)
	}
	return s.httpServer.ListenAndServe()
}

func (s *Server) Shutdown(ctx context.Context) error {
	return s.httpServer.Shutdown(ctx)
}

func (s *Server) routes() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", s.health)
	mux.HandleFunc("GET /version", s.version)
	mux.HandleFunc("GET /v1/capabilities", s.capabilities)
	mux.HandleFunc("POST /v1/auth/login", s.login)
	mux.HandleFunc("POST /v1/auth/refresh", s.refresh)
	mux.HandleFunc("DELETE /v1/auth/logout", s.logout)
	mux.HandleFunc("GET /v1/ws", s.websocket)
	return s.cors(mux)
}

func (s *Server) health(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

func (s *Server) version(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, version.Current())
}

func (s *Server) capabilities(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{
		"presence":                 true,
		"sharedQueue":              true,
		"sharedPlayback":           true,
		"playbackTransfer":         true,
		"protocolVersion":          version.ProtocolVersion,
		"minClientProtocolVersion": version.ProtocolVersion,
	})
}

func (s *Server) login(w http.ResponseWriter, r *http.Request) {
	var req LoginRequest
	if !readJSON(w, r, &req) {
		return
	}
	ctx := r.Context()
	user, err := s.upstream.Authenticate(ctx, req.UpstreamServerURL, req.Auth)
	if err != nil {
		s.writeUpstreamError(w, err)
		return
	}
	if !s.config.UpstreamAllowed(user.NormalizedServerURL) {
		writeError(w, http.StatusForbidden, "upstream server is not allowlisted", 0, "")
		return
	}

	deviceID := strings.TrimSpace(req.Device.DeviceID)
	if deviceID == "" {
		deviceID = newID()
	}
	displayName := strings.TrimSpace(req.Device.DisplayName)
	if displayName == "" {
		displayName = deviceID
	}
	userKey := opensubsonic.UserKey(user.NormalizedServerURL, user.Username)
	if err := s.store.UpsertDevice(ctx, store.Device{
		DeviceID:          deviceID,
		UserKey:           userKey,
		UpstreamServerURL: user.NormalizedServerURL,
		Username:          user.Username,
		DisplayName:       displayName,
		Platform:          strings.TrimSpace(req.Device.Platform),
		AppVersion:        strings.TrimSpace(req.Device.AppVersion),
		LastSeenAt:        time.Now().UTC(),
	}); err != nil {
		s.logger.Error("device upsert failed", "error", err)
		writeError(w, http.StatusInternalServerError, "failed to save device", 0, "")
		return
	}
	issued, err := s.auth.Issue(ctx, auth.IssueRequest{
		UserKey:           userKey,
		DeviceID:          deviceID,
		UpstreamServerURL: user.NormalizedServerURL,
		Username:          user.Username,
	})
	if err != nil {
		s.logger.Error("session issue failed", "error", err)
		writeError(w, http.StatusInternalServerError, "failed to issue session", 0, "")
		return
	}
	writeJSON(w, http.StatusOK, LoginResponse{
		AccessToken: issued.AccessToken,
		ExpiresAt:   issued.ExpiresAt,
		UserKey:     userKey,
		DeviceID:    deviceID,
	})
}

func (s *Server) refresh(w http.ResponseWriter, r *http.Request) {
	issued, err := s.auth.Refresh(r.Context(), r.Header.Get("Authorization"))
	if err != nil {
		writeError(w, http.StatusUnauthorized, "invalid or expired token", 0, "")
		return
	}
	writeJSON(w, http.StatusOK, LoginResponse{
		AccessToken: issued.AccessToken,
		ExpiresAt:   issued.ExpiresAt,
		UserKey:     issued.Session.UserKey,
		DeviceID:    issued.Session.DeviceID,
	})
}

func (s *Server) logout(w http.ResponseWriter, r *http.Request) {
	if err := s.auth.Logout(r.Context(), r.Header.Get("Authorization")); err != nil {
		s.logger.Warn("logout failed", "error", err)
	}
	w.WriteHeader(http.StatusNoContent)
}

func (s *Server) websocket(w http.ResponseWriter, r *http.Request) {
	session, err := s.auth.Authenticate(r.Context(), r.Header.Get("Authorization"))
	if err != nil {
		writeError(w, http.StatusUnauthorized, "invalid or expired token", 0, "")
		return
	}

	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		OriginPatterns: s.config.AllowedOrigins,
	})
	if err != nil {
		s.logger.Warn("websocket accept failed", "error", err)
		return
	}
	defer conn.CloseNow()

	client, err := s.hub.Register(context.Background(), *session)
	if err != nil {
		_ = conn.Close(websocket.StatusInternalError, "register failed")
		return
	}
	defer s.hub.Unregister(context.Background(), client)

	writerDone := make(chan struct{})
	go func() {
		defer close(writerDone)
		for message := range client.Send {
			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			err := wsjson.Write(ctx, conn, message)
			cancel()
			if err != nil {
				return
			}
		}
	}()

	pingDone := make(chan struct{})
	go func() {
		defer close(pingDone)
		ticker := time.NewTicker(30 * time.Second)
		defer ticker.Stop()
		for {
			select {
			case <-writerDone:
				return
			case <-ticker.C:
				ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
				err := conn.Ping(ctx)
				cancel()
				if err != nil {
					return
				}
			}
		}
	}()

	for {
		var message realtime.Envelope
		err := wsjson.Read(context.Background(), conn, &message)
		if err != nil {
			_ = conn.Close(websocket.StatusNormalClosure, "")
			return
		}
		message.DeviceID = session.DeviceID
		s.hub.Handle(context.Background(), client, message)
	}
}

func (s *Server) writeUpstreamError(w http.ResponseWriter, err error) {
	var upstreamErr *opensubsonic.Error
	if !errors.As(err, &upstreamErr) {
		writeError(w, http.StatusBadGateway, err.Error(), 0, "")
		return
	}
	status := http.StatusBadGateway
	switch upstreamErr.Kind {
	case opensubsonic.ErrorAuth:
		status = http.StatusUnauthorized
	case opensubsonic.ErrorUnsupportedAuth:
		status = http.StatusBadRequest
	case opensubsonic.ErrorNetwork:
		status = http.StatusBadGateway
	case opensubsonic.ErrorProtocol, opensubsonic.ErrorServer:
		status = http.StatusBadGateway
	}
	writeError(w, status, upstreamErr.Message, upstreamErr.Code, upstreamErr.HelpURL)
}

func (s *Server) cors(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		origin := r.Header.Get("Origin")
		if origin != "" && originAllowed(origin, s.config.AllowedOrigins) {
			w.Header().Set("Access-Control-Allow-Origin", origin)
			w.Header().Set("Vary", "Origin")
			w.Header().Set("Access-Control-Allow-Headers", "Authorization, Content-Type")
			w.Header().Set("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
		}
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

func originAllowed(origin string, patterns []string) bool {
	if len(patterns) == 0 {
		return false
	}
	for _, pattern := range patterns {
		if pattern == "*" || pattern == origin {
			return true
		}
		if strings.HasSuffix(pattern, ":*") && strings.HasPrefix(origin, strings.TrimSuffix(pattern, ":*")) {
			return true
		}
	}
	return false
}

func readJSON(w http.ResponseWriter, r *http.Request, dst any) bool {
	defer r.Body.Close()
	decoder := json.NewDecoder(http.MaxBytesReader(w, r.Body, 1<<20))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(dst); err != nil {
		writeError(w, http.StatusBadRequest, "invalid JSON body", 0, "")
		return false
	}
	return true
}

func writeJSON(w http.ResponseWriter, status int, value any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(value)
}

func writeError(w http.ResponseWriter, status int, message string, code int, helpURL string) {
	writeJSON(w, status, ErrorResponse{Error: message, Code: code, HelpURL: helpURL})
}

func newID() string {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		return strings.ReplaceAll(time.Now().UTC().Format("20060102150405.000000000"), ".", "")
	}
	return hex.EncodeToString(b[:])
}
