package store

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"testing"
	"time"
)

func TestMigrateCreatesSchema(t *testing.T) {
	ctx := context.Background()
	store, err := OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()

	version, err := store.SchemaVersion(ctx)
	if err != nil {
		t.Fatal(err)
	}
	if version != "2" {
		t.Fatalf("unexpected schema version %s", version)
	}
}

func TestSessionStoresOnlyTokenHash(t *testing.T) {
	ctx := context.Background()
	store, err := OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()

	sum := sha256.Sum256([]byte("secret-token"))
	tokenHash := hex.EncodeToString(sum[:])
	err = store.CreateSession(ctx, Session{
		TokenHash:         tokenHash,
		UserKey:           "user",
		DeviceID:          "device",
		UpstreamServerURL: "https://music.example/rest",
		Username:          "demo",
		ExpiresAt:         time.Now().Add(time.Hour),
		CreatedAt:         time.Now(),
	})
	if err != nil {
		t.Fatal(err)
	}

	session, err := store.FindSession(ctx, tokenHash)
	if err != nil {
		t.Fatal(err)
	}
	if session == nil || session.TokenHash != tokenHash {
		t.Fatalf("unexpected session: %#v", session)
	}
	if session.TokenHash == "secret-token" {
		t.Fatal("raw token was stored")
	}
}

func TestPlaybackSnapshotRejectsStaleSeq(t *testing.T) {
	ctx := context.Background()
	store, err := OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()

	first := PlaybackSnapshot{
		UserKey:        "user",
		Seq:            2,
		SourceDeviceID: "device-a",
		State:          json.RawMessage(`{"playingState":"playing"}`),
		PlayingState:   "playing",
		UpdatedAt:      time.Now(),
	}
	accepted, _, err := store.SavePlaybackSnapshot(ctx, first)
	if err != nil || !accepted {
		t.Fatalf("expected first snapshot accepted, accepted=%v err=%v", accepted, err)
	}

	stale := first
	stale.Seq = 1
	accepted, current, err := store.SavePlaybackSnapshot(ctx, stale)
	if err != nil {
		t.Fatal(err)
	}
	if accepted {
		t.Fatal("stale snapshot was accepted")
	}
	if current == nil || current.Seq != 2 || current.SourceDeviceID != "device-a" {
		t.Fatalf("unexpected current snapshot: %#v", current)
	}

	secondDevice := first
	secondDevice.SourceDeviceID = "device-b"
	secondDevice.Seq = 1
	accepted, _, err = store.SavePlaybackSnapshot(ctx, secondDevice)
	if err != nil || !accepted {
		t.Fatalf("expected second device snapshot accepted, accepted=%v err=%v", accepted, err)
	}
	snapshots, err := store.ListPlaybackSnapshots(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshots) != 2 {
		t.Fatalf("expected two device snapshots, got %d", len(snapshots))
	}
}
