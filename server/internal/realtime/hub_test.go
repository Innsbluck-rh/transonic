package realtime

import (
	"context"
	"encoding/json"
	"testing"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/store"
)

func TestRoomIsolationAndStaleBaseSeq(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()

	hub := NewHub(st, time.Minute)
	a := store.Session{UserKey: "user-a", DeviceID: "device-a", UpstreamServerURL: "https://a/rest", Username: "a"}
	b := store.Session{UserKey: "user-b", DeviceID: "device-b", UpstreamServerURL: "https://b/rest", Username: "b"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: a.UserKey, DeviceID: a.DeviceID, UpstreamServerURL: a.UpstreamServerURL, Username: a.Username, DisplayName: "A"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: b.UserKey, DeviceID: b.DeviceID, UpstreamServerURL: b.UpstreamServerURL, Username: b.Username, DisplayName: "B"})
	clientA, _ := hub.Register(ctx, a)
	clientB, _ := hub.Register(ctx, b)
	defer hub.Unregister(ctx, clientA)
	defer hub.Unregister(ctx, clientB)
	drain(clientA)
	drain(clientB)

	hub.Handle(ctx, clientA, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	gotA := <-clientA.Send
	if gotA.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected playback update for room A, got %s", gotA.Type)
	}
	select {
	case gotB := <-clientB.Send:
		t.Fatalf("room B received unexpected message: %#v", gotB)
	default:
	}

	hub.Handle(ctx, clientA, Envelope{
		ID:              "cmd-2",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-2","baseSeq":0,"op":"appendToQueue","items":[{"id":"song-b","title":"Song B"}]}`),
	})
	gotA = <-clientA.Send
	if gotA.Type != TypePlaybackCommandError {
		t.Fatalf("expected stale command to return error, got %s", gotA.Type)
	}
	gotA = <-clientA.Send
	if gotA.Type != TypePlaybackSharedSnapshot {
		t.Fatalf("expected stale command to return snapshot, got %s", gotA.Type)
	}
}

func TestPlaybackCommandRoutesToActiveDevice(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	activeSession := store.Session{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a"}
	remoteSession := store.Session{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Active"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Remote"})
	active, _ := hub.Register(ctx, activeSession)
	remote, _ := hub.Register(ctx, remoteSession)
	defer hub.Unregister(ctx, active)
	defer hub.Unregister(ctx, remote)
	drain(active)
	drain(remote)

	hub.Handle(ctx, active, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(active)
	drain(remote)

	hub.Handle(ctx, remote, Envelope{
		ID:              "cmd-2",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-2","baseSeq":1,"op":"play"}`),
	})
	forwarded := <-active.Send
	if forwarded.Type != TypePlaybackApply {
		t.Fatalf("expected active device apply, got %s", forwarded.Type)
	}
	select {
	case gotRemote := <-remote.Send:
		t.Fatalf("remote received unexpected message: %#v", gotRemote)
	default:
	}
}

func TestInsertAfterCurrentAppendsToPlayNextQueueArea(t *testing.T) {
	zero := int64(0)
	raw, err := encodePlaybackState(ConnectPlaybackState{
		PlayingState:      "stopped",
		Queue:             []json.RawMessage{json.RawMessage(`{"id":"song-a"}`), json.RawMessage(`{"id":"song-b"}`)},
		CurrentIndex:      &zero,
		CurrentPositionMs: 0,
	})
	if err != nil {
		t.Fatal(err)
	}
	current := &store.SharedPlaybackState{
		UserKey: "user",
		Seq:     1,
		State:   raw,
	}

	next, err := reduceQueueCommand(current, ConnectPlaybackCommand{
		Op:    "insertAfterCurrent",
		Items: []json.RawMessage{json.RawMessage(`{"id":"song-x"}`)},
	}, "device")
	if err != nil {
		t.Fatal(err)
	}
	next, err = reduceQueueCommand(next, ConnectPlaybackCommand{
		Op:    "insertAfterCurrent",
		Items: []json.RawMessage{json.RawMessage(`{"id":"song-y"}`)},
	}, "device")
	if err != nil {
		t.Fatal(err)
	}

	state, err := decodePlaybackState(next.State)
	if err != nil {
		t.Fatal(err)
	}
	if state.PlayNextQueueLen != 2 {
		t.Fatalf("expected play-next queue length 2, got %d", state.PlayNextQueueLen)
	}
	gotIDs := []string{}
	for index := range state.Queue {
		gotIDs = append(gotIDs, *songIDAt(state.Queue, int64(index)))
	}
	wantIDs := []string{"song-a", "song-x", "song-y", "song-b"}
	if len(gotIDs) != len(wantIDs) {
		t.Fatalf("unexpected queue ids: %#v", gotIDs)
	}
	for i := range wantIDs {
		if gotIDs[i] != wantIDs[i] {
			t.Fatalf("unexpected queue ids: %#v", gotIDs)
		}
	}
}

func TestInsertAfterCurrentIntoEmptyQueueInitializesPlayNextArea(t *testing.T) {
	raw, err := encodePlaybackState(emptyPlaybackState())
	if err != nil {
		t.Fatal(err)
	}
	current := &store.SharedPlaybackState{
		UserKey: "user",
		Seq:     1,
		State:   raw,
	}

	next, err := reduceQueueCommand(current, ConnectPlaybackCommand{
		Op:    "insertAfterCurrent",
		Items: []json.RawMessage{json.RawMessage(`{"id":"song-x"}`), json.RawMessage(`{"id":"song-y"}`)},
	}, "device")
	if err != nil {
		t.Fatal(err)
	}

	state, err := decodePlaybackState(next.State)
	if err != nil {
		t.Fatal(err)
	}
	if state.CurrentIndex == nil || *state.CurrentIndex != 0 {
		t.Fatalf("expected current index 0, got %#v", state.CurrentIndex)
	}
	if state.PlayNextQueueLen != 1 {
		t.Fatalf("expected play-next queue length 1, got %d", state.PlayNextQueueLen)
	}
	if got := songIDAt(state.Queue, 0); got == nil || *got != "song-x" {
		t.Fatalf("unexpected current song: %#v", got)
	}
}

func TestTransferPlaybackUpdatesActiveDevice(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	aSession := store.Session{UserKey: "user", DeviceID: "a", UpstreamServerURL: "https://a/rest", Username: "a"}
	bSession := store.Session{UserKey: "user", DeviceID: "b", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "a", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "A"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "b", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "B"})
	a, _ := hub.Register(ctx, aSession)
	b, _ := hub.Register(ctx, bSession)
	defer hub.Unregister(ctx, a)
	defer hub.Unregister(ctx, b)
	drain(a)
	drain(b)

	hub.Handle(ctx, a, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(a)
	drain(b)

	hub.Handle(ctx, b, Envelope{
		ID:              "cmd-2",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-2","baseSeq":1,"op":"transferPlayback","targetDeviceId":"b"}`),
	})
	updated := <-b.Send
	if updated.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected transfer update, got %s", updated.Type)
	}
	var payload struct {
		ActiveDeviceID string `json:"activeDeviceId"`
	}
	if err := json.Unmarshal(updated.Payload, &payload); err != nil {
		t.Fatal(err)
	}
	if payload.ActiveDeviceID != "b" {
		t.Fatalf("expected active device b, got %#v", payload)
	}
}

func TestTransferPlaybackRejectsUnavailableState(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	aSession := store.Session{UserKey: "user", DeviceID: "a", UpstreamServerURL: "https://a/rest", Username: "a"}
	bSession := store.Session{UserKey: "user", DeviceID: "b", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "a", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "A"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "b", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "B"})
	a, _ := hub.Register(ctx, aSession)
	b, _ := hub.Register(ctx, bSession)
	defer hub.Unregister(ctx, a)
	defer hub.Unregister(ctx, b)
	drain(a)
	drain(b)

	hub.Handle(ctx, b, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"transferPlayback","targetDeviceId":"b"}`),
	})
	if message := commandErrorMessage(t, b); message != "playback queue is empty" {
		t.Fatalf("unexpected transfer error: %s", message)
	}

	hub.Handle(ctx, a, Envelope{
		ID:              "cmd-2",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-2","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(a)
	drain(b)

	hub.Handle(ctx, b, Envelope{
		ID:              "cmd-3",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-3","baseSeq":1,"op":"transferPlayback","targetDeviceId":"a"}`),
	})
	if message := commandErrorMessage(t, b); message != "target device is already active" {
		t.Fatalf("unexpected transfer error: %s", message)
	}
}

func TestClearQueueEmptiesSharedPlaybackAndKeepsActiveDevice(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	activeSession := store.Session{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a"}
	remoteSession := store.Session{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Active"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Remote"})
	active, _ := hub.Register(ctx, activeSession)
	remote, _ := hub.Register(ctx, remoteSession)
	defer hub.Unregister(ctx, active)
	defer hub.Unregister(ctx, remote)
	drain(active)
	drain(remote)

	hub.Handle(ctx, active, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(active)
	drain(remote)

	hub.Handle(ctx, active, Envelope{
		ID:              "report-1",
		Type:            TypePlaybackReport,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"baseSeq":1,"state":{"playingState":"playing","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0,"currentPositionMs":42,"currentSongId":"song-a"}}`),
	})
	drain(active)
	drain(remote)

	hub.Handle(ctx, remote, Envelope{
		ID:              "cmd-2",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-2","baseSeq":1,"op":"clearQueue"}`),
	})

	gotActive := <-active.Send
	if gotActive.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected shared playback update for active device, got %s", gotActive.Type)
	}
	select {
	case extra := <-active.Send:
		t.Fatalf("active device received unexpected extra message: %#v", extra)
	default:
	}

	current, err := st.GetSharedPlaybackState(ctx, "user")
	if err != nil {
		t.Fatal(err)
	}
	if current.ActiveDeviceID != "active" {
		t.Fatalf("expected active device to be preserved, got %q", current.ActiveDeviceID)
	}
	state, err := decodePlaybackState(current.State)
	if err != nil {
		t.Fatal(err)
	}
	if state.PlayingState != "idle" || len(state.Queue) != 0 || state.CurrentIndex != nil || state.CurrentSongID != nil || state.CurrentPositionMs != 0 {
		t.Fatalf("unexpected cleared state: %#v", state)
	}
}

func TestPlaybackReportSameBaseSeqDoesNotBecomeStale(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	session := store.Session{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Active"})
	active, _ := hub.Register(ctx, session)
	defer hub.Unregister(ctx, active)
	drain(active)

	hub.Handle(ctx, active, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(active)

	hub.Handle(ctx, active, Envelope{
		ID:              "report-1",
		Type:            TypePlaybackReport,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"baseSeq":1,"state":{"playingState":"interrupted","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0,"currentPositionMs":0,"currentSongId":"song-a"}}`),
	})
	got := <-active.Send
	if got.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected interrupted report update, got %s", got.Type)
	}

	hub.Handle(ctx, active, Envelope{
		ID:              "report-2",
		Type:            TypePlaybackReport,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"baseSeq":1,"state":{"playingState":"playing","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0,"currentPositionMs":0,"currentSongId":"song-a"}}`),
	})
	got = <-active.Send
	if got.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected playing report update, got %s", got.Type)
	}
	var payload struct {
		Seq   int64 `json:"seq"`
		State struct {
			PlayingState string `json:"playingState"`
		} `json:"state"`
	}
	if err := json.Unmarshal(got.Payload, &payload); err != nil {
		t.Fatal(err)
	}
	if payload.Seq != 1 || payload.State.PlayingState != "playing" {
		t.Fatalf("unexpected report payload: %#v", payload)
	}
}

func TestStalePlaybackReportDoesNotRollbackQueue(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	activeSession := store.Session{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a"}
	remoteSession := store.Session{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Active"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Remote"})
	active, _ := hub.Register(ctx, activeSession)
	remote, _ := hub.Register(ctx, remoteSession)
	defer hub.Unregister(ctx, active)
	defer hub.Unregister(ctx, remote)
	drain(active)
	drain(remote)

	hub.Handle(ctx, active, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(active)
	drain(remote)

	hub.Handle(ctx, remote, Envelope{
		ID:              "cmd-2",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-2","baseSeq":1,"op":"appendToQueue","items":[{"id":"song-b","title":"Song B"}]}`),
	})
	drain(active)
	drain(remote)

	hub.Handle(ctx, active, Envelope{
		ID:              "report-1",
		Type:            TypePlaybackReport,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"baseSeq":1,"state":{"playingState":"playing","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0,"currentPositionMs":10,"currentSongId":"song-a"}}`),
	})
	got := <-remote.Send
	if got.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected report update, got %s", got.Type)
	}
	var payload struct {
		Seq   int64 `json:"seq"`
		State struct {
			PlayingState string `json:"playingState"`
			Queue        []struct {
				ID string `json:"id"`
			} `json:"queue"`
		} `json:"state"`
	}
	if err := json.Unmarshal(got.Payload, &payload); err != nil {
		t.Fatal(err)
	}
	if payload.Seq != 2 || payload.State.PlayingState != "playing" || len(payload.State.Queue) != 2 || payload.State.Queue[1].ID != "song-b" {
		t.Fatalf("unexpected report payload: %#v", payload)
	}
}

func TestActiveOfflineClearsActiveDeviceAndStopsPlayback(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	activeSession := store.Session{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a"}
	remoteSession := store.Session{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Active"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Remote"})
	active, _ := hub.Register(ctx, activeSession)
	remote, _ := hub.Register(ctx, remoteSession)
	defer hub.Unregister(ctx, remote)
	drain(active)
	drain(remote)

	hub.Handle(ctx, active, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(active)
	drain(remote)
	hub.Handle(ctx, active, Envelope{
		ID:              "report-1",
		Type:            TypePlaybackReport,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"baseSeq":1,"state":{"playingState":"playing","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0,"currentPositionMs":10,"currentSongId":"song-a"}}`),
	})
	drain(active)
	drain(remote)

	hub.Unregister(ctx, active)
	got := <-remote.Send
	if got.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected active offline playback update, got %s", got.Type)
	}
	var payload struct {
		ActiveDeviceID string `json:"activeDeviceId"`
		State          struct {
			PlayingState string `json:"playingState"`
		} `json:"state"`
	}
	if err := json.Unmarshal(got.Payload, &payload); err != nil {
		t.Fatal(err)
	}
	if payload.ActiveDeviceID != "" || payload.State.PlayingState != "stopped" {
		t.Fatalf("unexpected offline payload: %#v", payload)
	}
}

func TestOldConnectionUnregisterDoesNotRemoveReplacement(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()

	hub := NewHub(st, time.Minute)
	session := store.Session{UserKey: "user", DeviceID: "device", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "device", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Device"})
	oldClient, _ := hub.Register(ctx, session)
	drain(oldClient)
	replacement, _ := hub.Register(ctx, session)
	drain(replacement)

	hub.Unregister(ctx, oldClient)
	if !hub.isDeviceOnline("user", "device") {
		t.Fatal("replacement connection was removed by stale unregister")
	}
	if !hub.sendToDevice("user", "device", TypePresenceUpdated, "", mustJSON(map[string]any{"devices": []store.Device{}})) {
		t.Fatal("replacement connection is not reachable")
	}
	got := <-replacement.Send
	if got.Type != TypePresenceUpdated {
		t.Fatalf("expected replacement to receive presence update, got %s", got.Type)
	}

	hub.Unregister(ctx, replacement)
	if hub.isDeviceOnline("user", "device") {
		t.Fatal("replacement connection remained online after unregister")
	}
}

func TestPlaybackCommandClaimsActiveDeviceWhenMissing(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	activeSession := store.Session{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a"}
	remoteSession := store.Session{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Active"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Remote"})
	active, _ := hub.Register(ctx, activeSession)
	remote, _ := hub.Register(ctx, remoteSession)
	defer hub.Unregister(ctx, remote)
	drain(active)
	drain(remote)

	hub.Handle(ctx, active, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(active)
	drain(remote)
	hub.Handle(ctx, active, Envelope{
		ID:              "report-1",
		Type:            TypePlaybackReport,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"baseSeq":1,"state":{"playingState":"playing","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0,"currentPositionMs":10,"currentSongId":"song-a"}}`),
	})
	drain(active)
	drain(remote)
	hub.Unregister(ctx, active)
	drain(remote)

	hub.Handle(ctx, remote, Envelope{
		ID:              "cmd-2",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-2","baseSeq":2,"op":"play"}`),
	})
	updated := <-remote.Send
	if updated.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected active claim update, got %s", updated.Type)
	}
	var payload struct {
		ActiveDeviceID string `json:"activeDeviceId"`
		Seq            int64  `json:"seq"`
	}
	if err := json.Unmarshal(updated.Payload, &payload); err != nil {
		t.Fatal(err)
	}
	if payload.ActiveDeviceID != "remote" || payload.Seq != 3 {
		t.Fatalf("unexpected claim payload: %#v", payload)
	}
	applied := <-remote.Send
	if applied.Type != TypePlaybackApply {
		t.Fatalf("expected playback apply after active claim, got %s", applied.Type)
	}
}

func TestTransferPlaybackClaimsTargetWhenActiveDeviceIsMissing(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	activeSession := store.Session{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a"}
	remoteSession := store.Session{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "active", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Active"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "remote", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Remote"})
	active, _ := hub.Register(ctx, activeSession)
	remote, _ := hub.Register(ctx, remoteSession)
	defer hub.Unregister(ctx, remote)
	drain(active)
	drain(remote)

	hub.Handle(ctx, active, Envelope{
		ID:              "cmd-1",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","baseSeq":0,"op":"setQueue","queue":[{"id":"song-a","title":"Song A"}],"currentIndex":0}`),
	})
	drain(active)
	drain(remote)
	hub.Unregister(ctx, active)
	drain(remote)

	hub.Handle(ctx, remote, Envelope{
		ID:              "cmd-2",
		Type:            TypePlaybackCommandRequest,
		ProtocolVersion: 2,
		Payload:         json.RawMessage(`{"commandId":"cmd-2","baseSeq":2,"op":"transferPlayback","targetDeviceId":"remote"}`),
	})
	updated := <-remote.Send
	if updated.Type != TypePlaybackSharedUpdated {
		t.Fatalf("expected transfer update, got %s", updated.Type)
	}
	var payload struct {
		ActiveDeviceID string `json:"activeDeviceId"`
	}
	if err := json.Unmarshal(updated.Payload, &payload); err != nil {
		t.Fatal(err)
	}
	if payload.ActiveDeviceID != "remote" {
		t.Fatalf("expected remote to become active, got %#v", payload)
	}
}

func commandErrorMessage(t *testing.T, client *Client) string {
	t.Helper()
	got := <-client.Send
	if got.Type != TypePlaybackCommandError {
		t.Fatalf("expected command error, got %s", got.Type)
	}
	var payload struct {
		Message string `json:"message"`
	}
	if err := json.Unmarshal(got.Payload, &payload); err != nil {
		t.Fatal(err)
	}
	return payload.Message
}

func drain(client *Client) {
	for {
		select {
		case <-client.Send:
		default:
			return
		}
	}
}
