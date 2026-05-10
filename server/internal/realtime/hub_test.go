package realtime

import (
	"context"
	"encoding/json"
	"testing"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/store"
)

func TestRoomIsolationAndStaleSnapshot(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()

	hub := NewHub(st, time.Minute)
	a := store.Session{UserKey: "user-a", DeviceID: "device-a", UpstreamServerURL: "https://a/rest", Username: "a"}
	b := store.Session{UserKey: "user-b", DeviceID: "device-b", UpstreamServerURL: "https://b/rest", Username: "b"}
	if err := st.UpsertDevice(ctx, store.Device{UserKey: a.UserKey, DeviceID: a.DeviceID, UpstreamServerURL: a.UpstreamServerURL, Username: a.Username, DisplayName: "A"}); err != nil {
		t.Fatal(err)
	}
	if err := st.UpsertDevice(ctx, store.Device{UserKey: b.UserKey, DeviceID: b.DeviceID, UpstreamServerURL: b.UpstreamServerURL, Username: b.Username, DisplayName: "B"}); err != nil {
		t.Fatal(err)
	}
	clientA, err := hub.Register(ctx, a)
	if err != nil {
		t.Fatal(err)
	}
	defer hub.Unregister(ctx, clientA)
	clientB, err := hub.Register(ctx, b)
	if err != nil {
		t.Fatal(err)
	}
	defer hub.Unregister(ctx, clientB)
	drain(clientA)
	drain(clientB)

	hub.Handle(ctx, clientA, Envelope{
		ID:              "1",
		Type:            TypePlaybackPublish,
		ProtocolVersion: 1,
		Payload:         json.RawMessage(`{"seq":2,"state":{"playingState":"playing","currentSongId":"song-a","currentIndex":0,"currentPositionMs":10}}`),
	})
	gotA := <-clientA.Send
	if gotA.Type != TypePlaybackUpdated {
		t.Fatalf("expected playback update for room A, got %s", gotA.Type)
	}
	select {
	case gotB := <-clientB.Send:
		t.Fatalf("room B received unexpected message: %#v", gotB)
	default:
	}

	hub.Handle(ctx, clientA, Envelope{
		ID:              "2",
		Type:            TypePlaybackPublish,
		ProtocolVersion: 1,
		Payload:         json.RawMessage(`{"seq":1,"state":{"playingState":"paused","currentPositionMs":0}}`),
	})
	gotA = <-clientA.Send
	if gotA.Type != TypePlaybackSnapshot {
		t.Fatalf("expected stale publish to return snapshot, got %s", gotA.Type)
	}
}

func TestRemoteCommandRoutesAck(t *testing.T) {
	ctx := context.Background()
	st, err := store.OpenMemory(ctx)
	if err != nil {
		t.Fatal(err)
	}
	defer st.Close()
	hub := NewHub(st, time.Minute)
	sourceSession := store.Session{UserKey: "user", DeviceID: "source", UpstreamServerURL: "https://a/rest", Username: "a"}
	targetSession := store.Session{UserKey: "user", DeviceID: "target", UpstreamServerURL: "https://a/rest", Username: "a"}
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "source", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Source"})
	_ = st.UpsertDevice(ctx, store.Device{UserKey: "user", DeviceID: "target", UpstreamServerURL: "https://a/rest", Username: "a", DisplayName: "Target"})
	source, _ := hub.Register(ctx, sourceSession)
	target, _ := hub.Register(ctx, targetSession)
	defer hub.Unregister(ctx, source)
	defer hub.Unregister(ctx, target)
	drain(source)
	drain(target)

	hub.Handle(ctx, source, Envelope{
		ID:              "cmd-1",
		Type:            TypeRemoteCommand,
		ProtocolVersion: 1,
		Payload:         json.RawMessage(`{"commandId":"cmd-1","targetDeviceId":"target","command":"pause"}`),
	})
	forwarded := <-target.Send
	if forwarded.Type != TypeRemoteCommand {
		t.Fatalf("expected target command, got %s", forwarded.Type)
	}
	hub.Handle(ctx, target, Envelope{
		ID:              "ack-1",
		Type:            TypeRemoteAck,
		ProtocolVersion: 1,
		Payload:         json.RawMessage(`{"commandId":"cmd-1"}`),
	})
	ack := <-source.Send
	if ack.Type != TypeRemoteAck {
		t.Fatalf("expected source ack, got %s", ack.Type)
	}
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
