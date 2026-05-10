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

type PlaybackSnapshot struct {
	UserKey           string          `json:"-"`
	Seq               int64           `json:"seq"`
	SourceDeviceID    string          `json:"sourceDeviceId"`
	State             json.RawMessage `json:"state"`
	PlayingState      string          `json:"playingState,omitempty"`
	CurrentSongID     string          `json:"currentSongId,omitempty"`
	CurrentIndex      *int64          `json:"currentIndex,omitempty"`
	CurrentPositionMs int64           `json:"currentPositionMs"`
	UpdatedAt         time.Time       `json:"updatedAt"`
}

type Handoff struct {
	HandoffID      string          `json:"handoffId"`
	UserKey        string          `json:"-"`
	SourceDeviceID string          `json:"sourceDeviceId"`
	TargetDeviceID string          `json:"targetDeviceId"`
	Status         string          `json:"status"`
	Snapshot       json.RawMessage `json:"snapshot,omitempty"`
	CreatedAt      time.Time       `json:"createdAt"`
	UpdatedAt      time.Time       `json:"updatedAt"`
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
	if err := s.migratePlaybackSnapshots(ctx); err != nil {
		return err
	}
	finalStatements := []string{
		`CREATE TABLE IF NOT EXISTS handoffs (
			handoff_id TEXT PRIMARY KEY,
			user_key TEXT NOT NULL,
			source_device_id TEXT NOT NULL,
			target_device_id TEXT NOT NULL,
			status TEXT NOT NULL,
			snapshot_json TEXT,
			created_at TEXT NOT NULL,
			updated_at TEXT NOT NULL
		)`,
		`INSERT INTO meta (key, value) VALUES ('schema_version', '2')
			ON CONFLICT(key) DO UPDATE SET value = excluded.value`,
	}
	for _, statement := range finalStatements {
		if _, err := s.db.ExecContext(ctx, statement); err != nil {
			return fmt.Errorf("migrate sqlite: %w", err)
		}
	}
	return nil
}

func (s *Store) migratePlaybackSnapshots(ctx context.Context) error {
	const createTable = `CREATE TABLE IF NOT EXISTS playback_snapshots (
		user_key TEXT NOT NULL,
		source_device_id TEXT NOT NULL,
		seq INTEGER NOT NULL,
		state_json TEXT NOT NULL,
		playing_state TEXT NOT NULL,
		current_song_id TEXT NOT NULL,
		current_index INTEGER,
		current_position_ms INTEGER NOT NULL,
		updated_at TEXT NOT NULL,
		PRIMARY KEY (user_key, source_device_id)
	)`

	var existing int
	if err := s.db.QueryRowContext(ctx, `SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'playback_snapshots'`).Scan(&existing); err != nil {
		return fmt.Errorf("inspect playback snapshots table: %w", err)
	}
	if existing == 0 {
		_, err := s.db.ExecContext(ctx, createTable)
		if err != nil {
			return fmt.Errorf("create playback snapshots table: %w", err)
		}
		return nil
	}

	rows, err := s.db.QueryContext(ctx, `PRAGMA table_info(playback_snapshots)`)
	if err != nil {
		return fmt.Errorf("inspect playback snapshots schema: %w", err)
	}
	defer rows.Close()
	sourceDeviceIsPK := false
	for rows.Next() {
		var cid int
		var name, columnType string
		var notNull, pk int
		var defaultValue any
		if err := rows.Scan(&cid, &name, &columnType, &notNull, &defaultValue, &pk); err != nil {
			return err
		}
		if name == "source_device_id" && pk > 0 {
			sourceDeviceIsPK = true
		}
	}
	if err := rows.Err(); err != nil {
		return err
	}
	if sourceDeviceIsPK {
		return nil
	}

	statements := []string{
		`DROP TABLE IF EXISTS playback_snapshots_migrate`,
		`CREATE TABLE playback_snapshots_migrate (
			user_key TEXT NOT NULL,
			source_device_id TEXT NOT NULL,
			seq INTEGER NOT NULL,
			state_json TEXT NOT NULL,
			playing_state TEXT NOT NULL,
			current_song_id TEXT NOT NULL,
			current_index INTEGER,
			current_position_ms INTEGER NOT NULL,
			updated_at TEXT NOT NULL,
			PRIMARY KEY (user_key, source_device_id)
		)`,
		`INSERT OR IGNORE INTO playback_snapshots_migrate (
			user_key, source_device_id, seq, state_json, playing_state, current_song_id, current_index, current_position_ms, updated_at
		)
		SELECT user_key, source_device_id, seq, state_json, playing_state, current_song_id, current_index, current_position_ms, updated_at
		FROM playback_snapshots`,
		`DROP TABLE playback_snapshots`,
		`ALTER TABLE playback_snapshots_migrate RENAME TO playback_snapshots`,
	}
	for _, statement := range statements {
		if _, err := s.db.ExecContext(ctx, statement); err != nil {
			return fmt.Errorf("migrate playback snapshots table: %w", err)
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
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return err
	}
	if _, err := tx.ExecContext(ctx, `DELETE FROM devices`); err != nil {
		_ = tx.Rollback()
		return err
	}
	if _, err := tx.ExecContext(ctx, `DELETE FROM playback_snapshots`); err != nil {
		_ = tx.Rollback()
		return err
	}
	return tx.Commit()
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
	if _, err := tx.ExecContext(ctx, `DELETE FROM playback_snapshots
		WHERE user_key = ?
		AND NOT EXISTS (
			SELECT 1 FROM devices
			WHERE devices.user_key = playback_snapshots.user_key
			AND devices.device_id = playback_snapshots.source_device_id
		)`, userKey); err != nil {
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

func (s *Store) SavePlaybackSnapshot(ctx context.Context, snapshot PlaybackSnapshot) (bool, *PlaybackSnapshot, error) {
	current, err := s.GetPlaybackSnapshot(ctx, snapshot.UserKey, snapshot.SourceDeviceID)
	if err != nil {
		return false, nil, err
	}
	if current != nil && snapshot.Seq <= current.Seq {
		return false, current, nil
	}
	if snapshot.UpdatedAt.IsZero() {
		snapshot.UpdatedAt = time.Now().UTC()
	}
	var currentIndex any
	if snapshot.CurrentIndex != nil {
		currentIndex = *snapshot.CurrentIndex
	}
	_, err = s.db.ExecContext(ctx, `INSERT INTO playback_snapshots (
		user_key, source_device_id, seq, state_json, playing_state, current_song_id, current_index, current_position_ms, updated_at
	) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(user_key, source_device_id) DO UPDATE SET
		seq = excluded.seq,
		state_json = excluded.state_json,
		playing_state = excluded.playing_state,
		current_song_id = excluded.current_song_id,
		current_index = excluded.current_index,
		current_position_ms = excluded.current_position_ms,
		updated_at = excluded.updated_at`,
		snapshot.UserKey,
		snapshot.SourceDeviceID,
		snapshot.Seq,
		string(snapshot.State),
		snapshot.PlayingState,
		snapshot.CurrentSongID,
		currentIndex,
		snapshot.CurrentPositionMs,
		formatTime(snapshot.UpdatedAt),
	)
	if err != nil {
		return false, nil, err
	}
	return true, &snapshot, nil
}

func (s *Store) GetPlaybackSnapshot(ctx context.Context, userKey, sourceDeviceID string) (*PlaybackSnapshot, error) {
	var snapshot PlaybackSnapshot
	var stateJSON string
	var currentIndex sql.NullInt64
	var updatedAt string
	err := s.db.QueryRowContext(ctx, `SELECT user_key, seq, source_device_id, state_json, playing_state, current_song_id, current_index, current_position_ms, updated_at
		FROM playback_snapshots WHERE user_key = ? AND source_device_id = ?`, userKey, sourceDeviceID).
		Scan(&snapshot.UserKey, &snapshot.Seq, &snapshot.SourceDeviceID, &stateJSON, &snapshot.PlayingState, &snapshot.CurrentSongID, &currentIndex, &snapshot.CurrentPositionMs, &updatedAt)
	if errors.Is(err, sql.ErrNoRows) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	if currentIndex.Valid {
		value := currentIndex.Int64
		snapshot.CurrentIndex = &value
	}
	snapshot.State = json.RawMessage(stateJSON)
	snapshot.UpdatedAt, _ = parseTime(updatedAt)
	return &snapshot, nil
}

func (s *Store) ListPlaybackSnapshots(ctx context.Context, userKey string) ([]PlaybackSnapshot, error) {
	rows, err := s.db.QueryContext(ctx, `SELECT user_key, seq, source_device_id, state_json, playing_state, current_song_id, current_index, current_position_ms, updated_at
		FROM playback_snapshots WHERE user_key = ? ORDER BY source_device_id`, userKey)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var snapshots []PlaybackSnapshot
	for rows.Next() {
		var snapshot PlaybackSnapshot
		var stateJSON string
		var currentIndex sql.NullInt64
		var updatedAt string
		if err := rows.Scan(&snapshot.UserKey, &snapshot.Seq, &snapshot.SourceDeviceID, &stateJSON, &snapshot.PlayingState, &snapshot.CurrentSongID, &currentIndex, &snapshot.CurrentPositionMs, &updatedAt); err != nil {
			return nil, err
		}
		if currentIndex.Valid {
			value := currentIndex.Int64
			snapshot.CurrentIndex = &value
		}
		snapshot.State = json.RawMessage(stateJSON)
		snapshot.UpdatedAt, _ = parseTime(updatedAt)
		snapshots = append(snapshots, snapshot)
	}
	return snapshots, rows.Err()
}

func (s *Store) UpsertHandoff(ctx context.Context, handoff Handoff) error {
	if handoff.CreatedAt.IsZero() {
		handoff.CreatedAt = time.Now().UTC()
	}
	if handoff.UpdatedAt.IsZero() {
		handoff.UpdatedAt = handoff.CreatedAt
	}
	var snapshot any
	if len(handoff.Snapshot) > 0 {
		snapshot = string(handoff.Snapshot)
	}
	_, err := s.db.ExecContext(ctx, `INSERT INTO handoffs (
		handoff_id, user_key, source_device_id, target_device_id, status, snapshot_json, created_at, updated_at
	) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(handoff_id) DO UPDATE SET
		status = excluded.status,
		snapshot_json = excluded.snapshot_json,
		updated_at = excluded.updated_at`,
		handoff.HandoffID,
		handoff.UserKey,
		handoff.SourceDeviceID,
		handoff.TargetDeviceID,
		handoff.Status,
		snapshot,
		formatTime(handoff.CreatedAt),
		formatTime(handoff.UpdatedAt),
	)
	return err
}

func (s *Store) UpdateHandoffStatus(ctx context.Context, userKey, handoffID, status string, snapshot json.RawMessage) error {
	var snapshotValue any
	if len(snapshot) > 0 {
		snapshotValue = string(snapshot)
	}
	result, err := s.db.ExecContext(ctx, `UPDATE handoffs SET status = ?, snapshot_json = COALESCE(?, snapshot_json), updated_at = ?
		WHERE user_key = ? AND handoff_id = ?`, status, snapshotValue, formatTime(time.Now().UTC()), userKey, handoffID)
	if err != nil {
		return err
	}
	count, err := result.RowsAffected()
	if err != nil {
		return err
	}
	if count == 0 {
		return sql.ErrNoRows
	}
	return nil
}

func formatTime(t time.Time) string {
	return t.UTC().Format(time.RFC3339Nano)
}

func parseTime(value string) (time.Time, error) {
	return time.Parse(time.RFC3339Nano, value)
}
