package store

import (
	"context"
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"time"

	_ "modernc.org/sqlite"
)

type Store struct {
	db *sql.DB
}

type Device struct {
	DeviceID          string    `json:"deviceId"`
	UserKey           string    `json:"-"`
	UpstreamServerURL string    `json:"upstreamServerUrl,omitempty"`
	Username          string    `json:"username,omitempty"`
	DisplayName       string    `json:"displayName,omitempty"`
	Platform          string    `json:"platform,omitempty"`
	AppVersion        string    `json:"appVersion,omitempty"`
	LastSeenAt        time.Time `json:"lastSeenAt"`
	Online            bool      `json:"online"`
}

type Session struct {
	TokenHash         string
	UserKey           string
	DeviceID          string
	UpstreamServerURL string
	Username          string
	ExpiresAt         time.Time
	CreatedAt         time.Time
}

type SharedPlaybackState struct {
	UserKey           string          `json:"-"`
	Seq               int64           `json:"seq"`
	ActiveDeviceID    string          `json:"activeDeviceId,omitempty"`
	State             json.RawMessage `json:"state"`
	UpdatedAt         time.Time       `json:"updatedAt"`
	UpdatedByDeviceID string          `json:"updatedByDeviceId,omitempty"`
}

func Open(ctx context.Context, dataDir string) (*Store, error) {
	if err := os.MkdirAll(dataDir, 0o750); err != nil {
		return nil, fmt.Errorf("create data dir: %w", err)
	}
	db, err := sql.Open("sqlite", filepath.Join(dataDir, "transonic-server.sqlite3"))
	if err != nil {
		return nil, fmt.Errorf("open sqlite: %w", err)
	}
	db.SetMaxOpenConns(1)
	store := &Store{db: db}
	if err := store.Migrate(ctx); err != nil {
		_ = db.Close()
		return nil, err
	}
	return store, nil
}

func OpenMemory(ctx context.Context) (*Store, error) {
	db, err := sql.Open("sqlite", "file:transonic-server-test?mode=memory&cache=shared")
	if err != nil {
		return nil, err
	}
	db.SetMaxOpenConns(1)
	store := &Store{db: db}
	if err := store.Migrate(ctx); err != nil {
		_ = db.Close()
		return nil, err
	}
	return store, nil
}

func (s *Store) Close() error {
	if s == nil || s.db == nil {
		return nil
	}
	return s.db.Close()
}

func (s *Store) Migrate(ctx context.Context) error {
	baseStatements := []string{
		`CREATE TABLE IF NOT EXISTS meta (
			key TEXT PRIMARY KEY,
			value TEXT NOT NULL
		)`,
		`CREATE TABLE IF NOT EXISTS devices (
			device_id TEXT NOT NULL,
			user_key TEXT NOT NULL,
			upstream_server_url TEXT NOT NULL,
			username TEXT NOT NULL,
			display_name TEXT NOT NULL,
			platform TEXT NOT NULL,
			app_version TEXT NOT NULL,
			last_seen_at TEXT NOT NULL,
			PRIMARY KEY (user_key, device_id)
		)`,
		`CREATE TABLE IF NOT EXISTS sessions (
			token_hash TEXT PRIMARY KEY,
			user_key TEXT NOT NULL,
			device_id TEXT NOT NULL,
			upstream_server_url TEXT NOT NULL,
			username TEXT NOT NULL,
			expires_at TEXT NOT NULL,
			created_at TEXT NOT NULL
		)`,
		`CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions (expires_at)`,
	}
	for _, statement := range baseStatements {
		if _, err := s.db.ExecContext(ctx, statement); err != nil {
			return fmt.Errorf("migrate sqlite: %w", err)
		}
	}
	if err := s.migrateSharedPlaybackStates(ctx); err != nil {
		return err
	}
	finalStatements := []string{
		`DROP TABLE IF EXISTS playback_snapshots`,
		`DROP TABLE IF EXISTS handoffs`,
		`INSERT INTO meta (key, value) VALUES ('schema_version', '3')
			ON CONFLICT(key) DO UPDATE SET value = excluded.value`,
	}
	for _, statement := range finalStatements {
		if _, err := s.db.ExecContext(ctx, statement); err != nil {
			return fmt.Errorf("migrate sqlite: %w", err)
		}
	}
	return nil
}

func (s *Store) migrateSharedPlaybackStates(ctx context.Context) error {
	statements := []string{
		`CREATE TABLE IF NOT EXISTS shared_playback_states (
			user_key TEXT PRIMARY KEY,
			seq INTEGER NOT NULL,
			active_device_id TEXT NOT NULL,
			state_json TEXT NOT NULL,
			updated_at TEXT NOT NULL,
			updated_by_device_id TEXT NOT NULL
		)`,
		`DROP TABLE IF EXISTS playback_snapshots`,
		`DROP TABLE IF EXISTS handoffs`,
	}
	for _, statement := range statements {
		if _, err := s.db.ExecContext(ctx, statement); err != nil {
			return fmt.Errorf("migrate shared playback states: %w", err)
		}
	}
	return nil
}

func (s *Store) SchemaVersion(ctx context.Context) (string, error) {
	var version string
	err := s.db.QueryRowContext(ctx, `SELECT value FROM meta WHERE key = 'schema_version'`).Scan(&version)
	return version, err
}

func (s *Store) UpsertDevice(ctx context.Context, device Device) error {
	if device.LastSeenAt.IsZero() {
		device.LastSeenAt = time.Now().UTC()
	}
	_, err := s.db.ExecContext(ctx, `INSERT INTO devices (
		device_id, user_key, upstream_server_url, username, display_name, platform, app_version, last_seen_at
	) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(user_key, device_id) DO UPDATE SET
		upstream_server_url = excluded.upstream_server_url,
		username = excluded.username,
		display_name = excluded.display_name,
		platform = excluded.platform,
		app_version = excluded.app_version,
		last_seen_at = excluded.last_seen_at`,
		device.DeviceID,
		device.UserKey,
		device.UpstreamServerURL,
		device.Username,
		device.DisplayName,
		device.Platform,
		device.AppVersion,
		formatTime(device.LastSeenAt),
	)
	return err
}

func (s *Store) ClearPresence(ctx context.Context) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM devices`)
	return err
}

func (s *Store) DeleteStaleDevices(ctx context.Context, userKey string, cutoff time.Time, keepDeviceIDs []string) error {
	query := `DELETE FROM devices WHERE user_key = ? AND last_seen_at < ?`
	args := []any{userKey, formatTime(cutoff)}
	for _, deviceID := range keepDeviceIDs {
		query += ` AND device_id <> ?`
		args = append(args, deviceID)
	}

	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return err
	}
	if _, err := tx.ExecContext(ctx, query, args...); err != nil {
		_ = tx.Rollback()
		return err
	}
	return tx.Commit()
}

func (s *Store) ListDevices(ctx context.Context, userKey string) ([]Device, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT device_id, user_key, upstream_server_url, username, display_name, platform, app_version, last_seen_at
		FROM devices WHERE user_key = ? ORDER BY display_name, device_id`, userKey)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var devices []Device
	for rows.Next() {
		var device Device
		var lastSeen string
		if err := rows.Scan(&device.DeviceID, &device.UserKey, &device.UpstreamServerURL, &device.Username, &device.DisplayName, &device.Platform, &device.AppVersion, &lastSeen); err != nil {
			return nil, err
		}
		device.LastSeenAt, _ = parseTime(lastSeen)
		devices = append(devices, device)
	}
	return devices, rows.Err()
}

func (s *Store) CreateSession(ctx context.Context, session Session) error {
	_, err := s.db.ExecContext(ctx, `INSERT INTO sessions (
		token_hash, user_key, device_id, upstream_server_url, username, expires_at, created_at
	) VALUES (?, ?, ?, ?, ?, ?, ?)`,
		session.TokenHash,
		session.UserKey,
		session.DeviceID,
		session.UpstreamServerURL,
		session.Username,
		formatTime(session.ExpiresAt),
		formatTime(session.CreatedAt),
	)
	return err
}

func (s *Store) FindSession(ctx context.Context, tokenHash string) (*Session, error) {
	var session Session
	var expiresAt string
	var createdAt string
	err := s.db.QueryRowContext(ctx, `SELECT token_hash, user_key, device_id, upstream_server_url, username, expires_at, created_at
		FROM sessions WHERE token_hash = ?`, tokenHash).
		Scan(&session.TokenHash, &session.UserKey, &session.DeviceID, &session.UpstreamServerURL, &session.Username, &expiresAt, &createdAt)
	if errors.Is(err, sql.ErrNoRows) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	session.ExpiresAt, _ = parseTime(expiresAt)
	session.CreatedAt, _ = parseTime(createdAt)
	return &session, nil
}

func (s *Store) DeleteSession(ctx context.Context, tokenHash string) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM sessions WHERE token_hash = ?`, tokenHash)
	return err
}

func (s *Store) DeleteExpiredSessions(ctx context.Context, now time.Time) error {
	_, err := s.db.ExecContext(ctx, `DELETE FROM sessions WHERE expires_at <= ?`, formatTime(now))
	return err
}

func (s *Store) SaveSharedPlaybackState(ctx context.Context, state SharedPlaybackState) error {
	if state.UpdatedAt.IsZero() {
		state.UpdatedAt = time.Now().UTC()
	}
	_, err := s.db.ExecContext(ctx, `INSERT INTO shared_playback_states (
		user_key, seq, active_device_id, state_json, updated_at, updated_by_device_id
	) VALUES (?, ?, ?, ?, ?, ?)
	ON CONFLICT(user_key) DO UPDATE SET
		seq = excluded.seq,
		active_device_id = excluded.active_device_id,
		state_json = excluded.state_json,
		updated_at = excluded.updated_at,
		updated_by_device_id = excluded.updated_by_device_id`,
		state.UserKey,
		state.Seq,
		state.ActiveDeviceID,
		string(state.State),
		formatTime(state.UpdatedAt),
		state.UpdatedByDeviceID,
	)
	return err
}

func (s *Store) GetSharedPlaybackState(ctx context.Context, userKey string) (*SharedPlaybackState, error) {
	var state SharedPlaybackState
	var stateJSON string
	var updatedAt string
	err := s.db.QueryRowContext(ctx, `SELECT user_key, seq, active_device_id, state_json, updated_at, updated_by_device_id
		FROM shared_playback_states WHERE user_key = ?`, userKey).
		Scan(&state.UserKey, &state.Seq, &state.ActiveDeviceID, &stateJSON, &updatedAt, &state.UpdatedByDeviceID)
	if errors.Is(err, sql.ErrNoRows) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	state.State = json.RawMessage(stateJSON)
	state.UpdatedAt, _ = parseTime(updatedAt)
	return &state, nil
}

func formatTime(t time.Time) string {
	return t.UTC().Format(time.RFC3339Nano)
}

func parseTime(value string) (time.Time, error) {
	return time.Parse(time.RFC3339Nano, value)
}
