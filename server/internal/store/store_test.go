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

func TestClearPresenceDeletesDevicesAndPlaybackSnapshots(t *testing.T) {
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
	if accepted, _, err := store.SavePlaybackSnapshot(ctx, PlaybackSnapshot{
		UserKey:        "user",
		Seq:            1,
		SourceDeviceID: "device",
		State:          json.RawMessage(`{"playingState":"playing"}`),
		PlayingState:   "playing",
	}); err != nil || !accepted {
		t.Fatalf("expected snapshot accepted, accepted=%v err=%v", accepted, err)
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
	snapshots, err := store.ListPlaybackSnapshots(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshots) != 0 {
		t.Fatalf("expected playback snapshots cleared, got %d", len(snapshots))
	}
}

func TestDeleteStaleDevicesRemovesPlaybackSnapshotsButKeepsOnlineDevices(t *testing.T) {
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
		if accepted, _, err := store.SavePlaybackSnapshot(ctx, PlaybackSnapshot{
			UserKey:        device.UserKey,
			Seq:            1,
			SourceDeviceID: device.DeviceID,
			State:          json.RawMessage(`{"playingState":"playing"}`),
			PlayingState:   "playing",
		}); err != nil || !accepted {
			t.Fatalf("expected snapshot accepted for %s, accepted=%v err=%v", device.DeviceID, accepted, err)
		}
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
	snapshots, err := store.ListPlaybackSnapshots(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	gotSnapshots := map[string]bool{}
	for _, snapshot := range snapshots {
		gotSnapshots[snapshot.SourceDeviceID] = true
	}
	if gotSnapshots["offline-old"] || !gotSnapshots["online-old"] || !gotSnapshots["fresh"] {
		t.Fatalf("unexpected snapshots after cleanup: %#v", gotSnapshots)
	}
}
