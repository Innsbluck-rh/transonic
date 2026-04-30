package auth

import (
	"context"
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/store"
)

var ErrInvalidToken = errors.New("invalid or expired token")

type Manager struct {
	store  *store.Store
	secret []byte
	ttl    time.Duration
}

type IssueRequest struct {
	UserKey           string
	DeviceID          string
	UpstreamServerURL string
	Username          string
}

type IssuedToken struct {
	AccessToken string        `json:"accessToken"`
	ExpiresAt   time.Time     `json:"expiresAt"`
	Session     store.Session `json:"-"`
}

func NewManager(store *store.Store, secret []byte, ttl time.Duration) (*Manager, error) {
	if len(secret) < 16 {
		return nil, errors.New("session signing secret must be at least 16 bytes")
	}
	if ttl <= 0 {
		return nil, errors.New("session token TTL must be positive")
	}
	return &Manager{store: store, secret: secret, ttl: ttl}, nil
}

func (m *Manager) Issue(ctx context.Context, req IssueRequest) (*IssuedToken, error) {
	if req.UserKey == "" || req.DeviceID == "" || req.UpstreamServerURL == "" || req.Username == "" {
		return nil, errors.New("userKey, deviceId, upstreamServerUrl, and username are required")
	}

	raw, err := randomToken()
	if err != nil {
		return nil, err
	}
	token := raw + "." + m.sign(raw)
	now := time.Now().UTC()
	session := store.Session{
		TokenHash:         HashToken(token),
		UserKey:           req.UserKey,
		DeviceID:          req.DeviceID,
		UpstreamServerURL: req.UpstreamServerURL,
		Username:          req.Username,
		ExpiresAt:         now.Add(m.ttl),
		CreatedAt:         now,
	}
	if err := m.store.CreateSession(ctx, session); err != nil {
		return nil, err
	}
	return &IssuedToken{AccessToken: token, ExpiresAt: session.ExpiresAt, Session: session}, nil
}

func (m *Manager) Authenticate(ctx context.Context, token string) (*store.Session, error) {
	token = BearerToken(token)
	if token == "" || !m.verify(token) {
		return nil, ErrInvalidToken
	}
	session, err := m.store.FindSession(ctx, HashToken(token))
	if err != nil {
		return nil, err
	}
	if session == nil || !session.ExpiresAt.After(time.Now().UTC()) {
		return nil, ErrInvalidToken
	}
	return session, nil
}

func (m *Manager) Refresh(ctx context.Context, token string) (*IssuedToken, error) {
	session, err := m.Authenticate(ctx, token)
	if err != nil {
		return nil, err
	}
	if err := m.store.DeleteSession(ctx, session.TokenHash); err != nil {
		return nil, err
	}
	return m.Issue(ctx, IssueRequest{
		UserKey:           session.UserKey,
		DeviceID:          session.DeviceID,
		UpstreamServerURL: session.UpstreamServerURL,
		Username:          session.Username,
	})
}

func (m *Manager) Logout(ctx context.Context, token string) error {
	token = BearerToken(token)
	if token == "" {
		return nil
	}
	return m.store.DeleteSession(ctx, HashToken(token))
}

func BearerToken(value string) string {
	trimmed := strings.TrimSpace(value)
	if strings.HasPrefix(strings.ToLower(trimmed), "bearer ") {
		return strings.TrimSpace(trimmed[len("bearer "):])
	}
	return trimmed
}

func HashToken(token string) string {
	sum := sha256.Sum256([]byte(token))
	return hex.EncodeToString(sum[:])
}

func randomToken() (string, error) {
	var b [32]byte
	if _, err := rand.Read(b[:]); err != nil {
		return "", fmt.Errorf("generate token: %w", err)
	}
	return base64.RawURLEncoding.EncodeToString(b[:]), nil
}

func (m *Manager) sign(raw string) string {
	mac := hmac.New(sha256.New, m.secret)
	_, _ = mac.Write([]byte(raw))
	return base64.RawURLEncoding.EncodeToString(mac.Sum(nil))
}

func (m *Manager) verify(token string) bool {
	raw, signature, ok := strings.Cut(token, ".")
	if !ok || raw == "" || signature == "" {
		return false
	}
	expected := m.sign(raw)
	if len(expected) != len(signature) {
		return false
	}
	return subtle.ConstantTimeCompare([]byte(expected), []byte(signature)) == 1
}
