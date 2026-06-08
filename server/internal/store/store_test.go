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
	if version != "3" {
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

func TestSharedPlaybackStatePersistsByUser(t *testing.T) {
	ctx := context.Background()
	store, err := OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()

	first := SharedPlaybackState{
		UserKey:           "user",
		Seq:               1,
		ActiveDeviceID:    "device-a",
		State:             json.RawMessage(`{"playingState":"playing","queue":[],"currentIndex":null,"currentPositionMs":10,"currentSongId":null}`),
		UpdatedAt:         time.Now(),
		UpdatedByDeviceID: "device-a",
	}
	if err := store.SaveSharedPlaybackState(ctx, first); err != nil {
		t.Fatal(err)
	}
	second := first
	second.Seq = 2
	second.ActiveDeviceID = "device-b"
	second.UpdatedByDeviceID = "device-b"
	if err := store.SaveSharedPlaybackState(ctx, second); err != nil {
		t.Fatal(err)
	}
	current, err := store.GetSharedPlaybackState(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	if current == nil || current.Seq != 2 || current.ActiveDeviceID != "device-b" {
		t.Fatalf("unexpected shared playback state: %#v", current)
	}
}

func TestClearPresenceDeletesDevicesButPreservesSharedPlayback(t *testing.T) {
	ctx := context.Background()
	store, err := OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()

	if err := store.UpsertDevice(ctx, Device{
		UserKey:           "user",
		DeviceID:          "device",
		UpstreamServerURL: "https://music.example/rest",
		Username:          "demo",
		DisplayName:       "Desktop",
	}); err != nil {
		t.Fatal(err)
	}
	if err := store.SaveSharedPlaybackState(ctx, SharedPlaybackState{
		UserKey:           "user",
		Seq:               1,
		ActiveDeviceID:    "device",
		State:             json.RawMessage(`{"playingState":"playing","queue":[],"currentIndex":null,"currentPositionMs":10,"currentSongId":null}`),
		UpdatedByDeviceID: "device",
	}); err != nil {
		t.Fatal(err)
	}

	if err := store.ClearPresence(ctx); err != nil {
		t.Fatal(err)
	}
	devices, err := store.ListDevices(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	if len(devices) != 0 {
		t.Fatalf("expected devices cleared, got %d", len(devices))
	}
	shared, err := store.GetSharedPlaybackState(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	if shared == nil || shared.Seq != 1 {
		t.Fatalf("expected shared playback preserved, got %#v", shared)
	}
}

func TestDeleteStaleDevicesKeepsSharedPlaybackState(t *testing.T) {
	ctx := context.Background()
	store, err := OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()

	old := time.Now().Add(-time.Hour)
	for _, device := range []Device{
		{UserKey: "user", DeviceID: "offline-old", UpstreamServerURL: "https://music.example/rest", Username: "demo", DisplayName: "Old", LastSeenAt: old},
		{UserKey: "user", DeviceID: "online-old", UpstreamServerURL: "https://music.example/rest", Username: "demo", DisplayName: "Online", LastSeenAt: old},
		{UserKey: "user", DeviceID: "fresh", UpstreamServerURL: "https://music.example/rest", Username: "demo", DisplayName: "Fresh", LastSeenAt: time.Now()},
	} {
		if err := store.UpsertDevice(ctx, device); err != nil {
			t.Fatal(err)
		}
	}
	if err := store.SaveSharedPlaybackState(ctx, SharedPlaybackState{
		UserKey:           "user",
		Seq:               1,
		ActiveDeviceID:    "offline-old",
		State:             json.RawMessage(`{"playingState":"playing","queue":[],"currentIndex":null,"currentPositionMs":10,"currentSongId":null}`),
		UpdatedByDeviceID: "offline-old",
	}); err != nil {
		t.Fatal(err)
	}

	if err := store.DeleteStaleDevices(ctx, "user", time.Now().Add(-time.Minute), []string{"online-old"}); err != nil {
		t.Fatal(err)
	}
	devices, err := store.ListDevices(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	gotDevices := map[string]bool{}
	for _, device := range devices {
		gotDevices[device.DeviceID] = true
	}
	if gotDevices["offline-old"] || !gotDevices["online-old"] || !gotDevices["fresh"] {
		t.Fatalf("unexpected devices after cleanup: %#v", gotDevices)
	}
	shared, err := store.GetSharedPlaybackState(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	if shared == nil || shared.ActiveDeviceID != "offline-old" {
		t.Fatalf("expected shared playback state preserved, got %#v", shared)
	}
}
