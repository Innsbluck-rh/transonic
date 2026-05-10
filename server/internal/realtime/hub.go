package realtime

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/Innsbluck-rh/transonic/server/internal/store"
	"github.com/Innsbluck-rh/transonic/server/internal/version"
)

const (
	TypePresenceHello     = "presence.hello"
	TypePresenceHeartbeat = "presence.heartbeat"
	TypePresenceUpdated   = "presence.updated"

	TypePlaybackPublish  = "playback.state.publish"
	TypePlaybackSnapshot = "playback.state.snapshot"
	TypePlaybackUpdated  = "playback.state.updated"

	TypeHandoffRequest  = "handoff.request"
	TypeHandoffAccept   = "handoff.accept"
	TypeHandoffReject   = "handoff.reject"
	TypeHandoffComplete = "handoff.complete"

	TypeRemoteCommand = "remote.command"
	TypeRemoteAck     = "remote.ack"
	TypeRemoteError   = "remote.error"
)

type Envelope struct {
	ID              string          `json:"id,omitempty"`
	Type            string          `json:"type"`
	ProtocolVersion int             `json:"protocolVersion"`
	SentAt          time.Time       `json:"sentAt"`
	DeviceID        string          `json:"deviceId"`
	Payload         json.RawMessage `json:"payload,omitempty"`
}

type Client struct {
	Session store.Session
	Send    chan Envelope
}

type Hub struct {
	store           *store.Store
	presenceTimeout time.Duration

	mu           sync.Mutex
	clients      map[string]map[string]*Client
	remoteRoutes map[string]remoteRoute
}

type remoteRoute struct {
	UserKey        string
	SourceDeviceID string
}

func NewHub(store *store.Store, presenceTimeout time.Duration) *Hub {
	return &Hub{
		store:           store,
		presenceTimeout: presenceTimeout,
		clients:         make(map[string]map[string]*Client),
		remoteRoutes:    make(map[string]remoteRoute),
	}
}

func (h *Hub) Register(ctx context.Context, session store.Session) (*Client, error) {
	client := &Client{
		Session: session,
		Send:    make(chan Envelope, 32),
	}
	h.mu.Lock()
	if h.clients[session.UserKey] == nil {
		h.clients[session.UserKey] = make(map[string]*Client)
	}
	h.clients[session.UserKey][session.DeviceID] = client
	h.mu.Unlock()

	_ = h.touchDevice(ctx, session)
	h.broadcastPresence(ctx, session.UserKey)
	if snapshots, err := h.store.ListPlaybackSnapshots(ctx, session.UserKey); err == nil && len(snapshots) > 0 {
		h.send(client, TypePlaybackSnapshot, "", snapshotsPayload(snapshots))
	}
	return client, nil
}

func (h *Hub) Unregister(ctx context.Context, client *Client) {
	h.mu.Lock()
	if byDevice := h.clients[client.Session.UserKey]; byDevice != nil {
		delete(byDevice, client.Session.DeviceID)
		if len(byDevice) == 0 {
			delete(h.clients, client.Session.UserKey)
		}
	}
	close(client.Send)
	h.mu.Unlock()
	h.broadcastPresence(ctx, client.Session.UserKey)
}

func (h *Hub) Handle(ctx context.Context, client *Client, inbound Envelope) {
	if inbound.ProtocolVersion != 0 && inbound.ProtocolVersion != version.ProtocolVersion {
		h.sendError(client, inbound.ID, "protocol version is not supported")
		return
	}

	switch inbound.Type {
	case TypePresenceHello, TypePresenceHeartbeat:
		_ = h.touchDevice(ctx, client.Session)
		h.broadcastPresence(ctx, client.Session.UserKey)
	case TypePlaybackPublish:
		h.handlePlaybackPublish(ctx, client, inbound)
	case TypeHandoffRequest:
		h.handleHandoffRequest(ctx, client, inbound)
	case TypeHandoffAccept:
		h.handleHandoffDecision(ctx, client, inbound, "accepted", TypeHandoffAccept)
	case TypeHandoffReject:
		h.handleHandoffDecision(ctx, client, inbound, "rejected", TypeHandoffReject)
	case TypeHandoffComplete:
		h.handleHandoffComplete(ctx, client, inbound)
	case TypeRemoteCommand:
		h.handleRemoteCommand(client, inbound)
	case TypeRemoteAck, TypeRemoteError:
		h.handleRemoteResult(client, inbound)
	default:
		h.sendError(client, inbound.ID, fmt.Sprintf("unsupported message type %q", inbound.Type))
	}
}

func (h *Hub) touchDevice(ctx context.Context, session store.Session) error {
	devices, err := h.store.ListDevices(ctx, session.UserKey)
	if err != nil {
		return err
	}
	device := store.Device{
		DeviceID:          session.DeviceID,
		UserKey:           session.UserKey,
		UpstreamServerURL: session.UpstreamServerURL,
		Username:          session.Username,
		DisplayName:       session.DeviceID,
		LastSeenAt:        time.Now().UTC(),
	}
	for _, current := range devices {
		if current.DeviceID == session.DeviceID {
			device.DisplayName = current.DisplayName
			device.Platform = current.Platform
			device.AppVersion = current.AppVersion
			break
		}
	}
	return h.store.UpsertDevice(ctx, device)
}

func (h *Hub) handlePlaybackPublish(ctx context.Context, client *Client, inbound Envelope) {
	var publish struct {
		Seq       int64           `json:"seq"`
		State     json.RawMessage `json:"state"`
		UpdatedAt *time.Time      `json:"updatedAt,omitempty"`
	}
	if err := json.Unmarshal(inbound.Payload, &publish); err != nil || publish.Seq <= 0 || len(publish.State) == 0 {
		h.sendError(client, inbound.ID, "playback.state.publish requires seq and state")
		return
	}
	probe, err := probePlaybackState(publish.State)
	if err != nil {
		h.sendError(client, inbound.ID, err.Error())
		return
	}
	updatedAt := time.Now().UTC()
	if publish.UpdatedAt != nil {
		updatedAt = publish.UpdatedAt.UTC()
	}
	snapshot := store.PlaybackSnapshot{
		UserKey:           client.Session.UserKey,
		Seq:               publish.Seq,
		SourceDeviceID:    client.Session.DeviceID,
		State:             publish.State,
		PlayingState:      probe.PlayingState,
		CurrentSongID:     probe.CurrentSongID,
		CurrentIndex:      probe.CurrentIndex,
		CurrentPositionMs: probe.CurrentPositionMs,
		UpdatedAt:         updatedAt,
	}
	accepted, current, err := h.store.SavePlaybackSnapshot(ctx, snapshot)
	if err != nil {
		h.sendError(client, inbound.ID, "failed to save playback snapshot")
		return
	}
	if !accepted {
		h.send(client, TypePlaybackSnapshot, inbound.ID, snapshotPayload(current))
		return
	}
	h.broadcast(client.Session.UserKey, TypePlaybackUpdated, inbound.ID, snapshotPayload(&snapshot))
}

type playbackStateProbe struct {
	PlayingState      string
	CurrentSongID     string
	CurrentIndex      *int64
	CurrentPositionMs int64
}

func probePlaybackState(raw json.RawMessage) (playbackStateProbe, error) {
	var state struct {
		PlayingState      string `json:"playingState"`
		CurrentSongID     string `json:"currentSongId"`
		CurrentIndex      *int64 `json:"currentIndex"`
		CurrentPositionMs int64  `json:"currentPositionMs"`
	}
	if err := json.Unmarshal(raw, &state); err != nil {
		return playbackStateProbe{}, errors.New("state must be a JSON object compatible with PlaybackStatus")
	}
	if state.PlayingState == "" {
		return playbackStateProbe{}, errors.New("state.playingState is required")
	}
	if state.CurrentPositionMs < 0 {
		return playbackStateProbe{}, errors.New("state.currentPositionMs must not be negative")
	}
	return playbackStateProbe{
		PlayingState:      state.PlayingState,
		CurrentSongID:     state.CurrentSongID,
		CurrentIndex:      state.CurrentIndex,
		CurrentPositionMs: state.CurrentPositionMs,
	}, nil
}

func (h *Hub) handleHandoffRequest(ctx context.Context, client *Client, inbound Envelope) {
	var request struct {
		HandoffID      string `json:"handoffId,omitempty"`
		SourceDeviceID string `json:"sourceDeviceId"`
		TargetDeviceID string `json:"targetDeviceId,omitempty"`
	}
	if err := json.Unmarshal(inbound.Payload, &request); err != nil || request.SourceDeviceID == "" {
		h.sendError(client, inbound.ID, "handoff.request requires sourceDeviceId")
		return
	}
	if request.TargetDeviceID == "" {
		request.TargetDeviceID = client.Session.DeviceID
	}
	if request.TargetDeviceID != client.Session.DeviceID {
		h.sendError(client, inbound.ID, "handoff target must be the requesting device")
		return
	}
	if request.HandoffID == "" {
		request.HandoffID = randomID()
	}
	snapshot, _ := h.store.GetPlaybackSnapshot(ctx, client.Session.UserKey, request.SourceDeviceID)
	payloadBytes := mustJSON(map[string]any{
		"handoffId":      request.HandoffID,
		"sourceDeviceId": request.SourceDeviceID,
		"targetDeviceId": request.TargetDeviceID,
		"snapshot":       snapshot,
	})
	_ = h.store.UpsertHandoff(ctx, store.Handoff{
		HandoffID:      request.HandoffID,
		UserKey:        client.Session.UserKey,
		SourceDeviceID: request.SourceDeviceID,
		TargetDeviceID: request.TargetDeviceID,
		Status:         "requested",
		Snapshot:       payloadBytes,
	})

	if h.sendToDevice(client.Session.UserKey, request.SourceDeviceID, TypeHandoffRequest, inbound.ID, payloadBytes) {
		return
	}
	_ = h.store.UpdateHandoffStatus(ctx, client.Session.UserKey, request.HandoffID, "completed_offline", payloadBytes)
	h.send(client, TypeHandoffComplete, inbound.ID, mustJSON(map[string]any{
		"handoffId":    request.HandoffID,
		"sourceOnline": false,
		"snapshot":     snapshot,
	}))
}

func (h *Hub) handleHandoffDecision(ctx context.Context, client *Client, inbound Envelope, status string, outboundType string) {
	var decision struct {
		HandoffID      string          `json:"handoffId"`
		TargetDeviceID string          `json:"targetDeviceId"`
		Snapshot       json.RawMessage `json:"snapshot,omitempty"`
		Reason         string          `json:"reason,omitempty"`
	}
	if err := json.Unmarshal(inbound.Payload, &decision); err != nil || decision.HandoffID == "" || decision.TargetDeviceID == "" {
		h.sendError(client, inbound.ID, "handoff decision requires handoffId and targetDeviceId")
		return
	}
	_ = h.store.UpdateHandoffStatus(ctx, client.Session.UserKey, decision.HandoffID, status, decision.Snapshot)
	h.sendToDevice(client.Session.UserKey, decision.TargetDeviceID, outboundType, inbound.ID, mustJSON(map[string]any{
		"handoffId":      decision.HandoffID,
		"sourceDeviceId": client.Session.DeviceID,
		"targetDeviceId": decision.TargetDeviceID,
		"snapshot":       jsonOrNil(decision.Snapshot),
		"reason":         decision.Reason,
	}))
}

func (h *Hub) handleHandoffComplete(ctx context.Context, client *Client, inbound Envelope) {
	var complete struct {
		HandoffID      string `json:"handoffId"`
		SourceDeviceID string `json:"sourceDeviceId"`
	}
	if err := json.Unmarshal(inbound.Payload, &complete); err != nil || complete.HandoffID == "" {
		h.sendError(client, inbound.ID, "handoff.complete requires handoffId")
		return
	}
	_ = h.store.UpdateHandoffStatus(ctx, client.Session.UserKey, complete.HandoffID, "completed", nil)
	if complete.SourceDeviceID != "" {
		h.sendToDevice(client.Session.UserKey, complete.SourceDeviceID, TypeRemoteCommand, inbound.ID, mustJSON(map[string]any{
			"commandId":      "handoff-" + complete.HandoffID,
			"sourceDeviceId": client.Session.DeviceID,
			"targetDeviceId": complete.SourceDeviceID,
			"command":        "pause",
			"reason":         "handoff.complete",
		}))
	}
}

func (h *Hub) handleRemoteCommand(client *Client, inbound Envelope) {
	var command struct {
		CommandID      string          `json:"commandId,omitempty"`
		TargetDeviceID string          `json:"targetDeviceId"`
		Command        string          `json:"command"`
		Args           json.RawMessage `json:"args,omitempty"`
	}
	if err := json.Unmarshal(inbound.Payload, &command); err != nil || command.TargetDeviceID == "" || command.Command == "" {
		h.sendError(client, inbound.ID, "remote.command requires targetDeviceId and command")
		return
	}
	if command.CommandID == "" {
		command.CommandID = inbound.ID
	}
	if command.CommandID == "" {
		command.CommandID = randomID()
	}
	h.mu.Lock()
	h.remoteRoutes[command.CommandID] = remoteRoute{UserKey: client.Session.UserKey, SourceDeviceID: client.Session.DeviceID}
	h.mu.Unlock()
	ok := h.sendToDevice(client.Session.UserKey, command.TargetDeviceID, TypeRemoteCommand, inbound.ID, mustJSON(map[string]any{
		"commandId":      command.CommandID,
		"sourceDeviceId": client.Session.DeviceID,
		"targetDeviceId": command.TargetDeviceID,
		"command":        command.Command,
		"args":           jsonOrNil(command.Args),
	}))
	if !ok {
		h.send(client, TypeRemoteError, inbound.ID, mustJSON(map[string]any{
			"commandId": command.CommandID,
			"message":   "target device is offline",
		}))
	}
}

func (h *Hub) handleRemoteResult(client *Client, inbound Envelope) {
	var result struct {
		CommandID string `json:"commandId"`
		Message   string `json:"message,omitempty"`
	}
	if err := json.Unmarshal(inbound.Payload, &result); err != nil || result.CommandID == "" {
		h.sendError(client, inbound.ID, "remote result requires commandId")
		return
	}
	h.mu.Lock()
	route, ok := h.remoteRoutes[result.CommandID]
	if ok {
		delete(h.remoteRoutes, result.CommandID)
	}
	h.mu.Unlock()
	if !ok || route.UserKey != client.Session.UserKey {
		return
	}
	h.sendToDevice(client.Session.UserKey, route.SourceDeviceID, inbound.Type, inbound.ID, inbound.Payload)
}

func (h *Hub) broadcastPresence(ctx context.Context, userKey string) {
	online := h.onlineDeviceIDs(userKey)
	if h.presenceTimeout > 0 {
		_ = h.store.DeleteStaleDevices(ctx, userKey, time.Now().UTC().Add(-h.presenceTimeout), online)
	}
	devices, err := h.store.ListDevices(ctx, userKey)
	if err != nil {
		return
	}
	onlineSet := make(map[string]bool, len(online))
	for _, deviceID := range online {
		onlineSet[deviceID] = true
	}
	for i := range devices {
		devices[i].Online = onlineSet[devices[i].DeviceID]
	}
	h.broadcast(userKey, TypePresenceUpdated, "", mustJSON(map[string]any{"devices": devices}))
}

func (h *Hub) onlineDeviceIDs(userKey string) []string {
	h.mu.Lock()
	defer h.mu.Unlock()
	ids := make([]string, 0, len(h.clients[userKey]))
	for deviceID := range h.clients[userKey] {
		ids = append(ids, deviceID)
	}
	return ids
}

func (h *Hub) broadcast(userKey, typ, replyTo string, payload json.RawMessage) {
	h.mu.Lock()
	clients := make([]*Client, 0, len(h.clients[userKey]))
	for _, client := range h.clients[userKey] {
		clients = append(clients, client)
	}
	h.mu.Unlock()
	for _, client := range clients {
		h.send(client, typ, replyTo, payload)
	}
}

func (h *Hub) sendToDevice(userKey, deviceID, typ, replyTo string, payload json.RawMessage) bool {
	h.mu.Lock()
	client := h.clients[userKey][deviceID]
	h.mu.Unlock()
	if client == nil {
		return false
	}
	h.send(client, typ, replyTo, payload)
	return true
}

func (h *Hub) send(client *Client, typ, replyTo string, payload json.RawMessage) {
	message := Envelope{
		ID:              replyTo,
		Type:            typ,
		ProtocolVersion: version.ProtocolVersion,
		SentAt:          time.Now().UTC(),
		DeviceID:        client.Session.DeviceID,
		Payload:         payload,
	}
	select {
	case client.Send <- message:
	default:
	}
}

func (h *Hub) sendError(client *Client, replyTo, message string) {
	h.send(client, TypeRemoteError, replyTo, mustJSON(map[string]any{"message": message}))
}

func snapshotPayload(snapshot *store.PlaybackSnapshot) json.RawMessage {
	if snapshot == nil {
		return mustJSON(map[string]any{"snapshot": nil})
	}
	return mustJSON(map[string]any{
		"seq":            snapshot.Seq,
		"sourceDeviceId": snapshot.SourceDeviceID,
		"state":          json.RawMessage(snapshot.State),
		"playingState":   snapshot.PlayingState,
		"currentSongId":  snapshot.CurrentSongID,
		"currentIndex":   snapshot.CurrentIndex,
		"positionMs":     estimatedPosition(*snapshot),
		"updatedAt":      snapshot.UpdatedAt,
	})
}

func snapshotsPayload(snapshots []store.PlaybackSnapshot) json.RawMessage {
	payloads := make([]json.RawMessage, 0, len(snapshots))
	for i := range snapshots {
		payloads = append(payloads, snapshotPayload(&snapshots[i]))
	}
	return mustJSON(map[string]any{"snapshots": payloads})
}

func estimatedPosition(snapshot store.PlaybackSnapshot) int64 {
	if snapshot.PlayingState != "playing" {
		return snapshot.CurrentPositionMs
	}
	elapsed := time.Since(snapshot.UpdatedAt).Milliseconds()
	if elapsed < 0 {
		elapsed = 0
	}
	return snapshot.CurrentPositionMs + elapsed
}

func mustJSON(value any) json.RawMessage {
	raw, err := json.Marshal(value)
	if err != nil {
		return json.RawMessage(`{"error":"json marshal failed"}`)
	}
	return raw
}

func jsonOrNil(raw json.RawMessage) any {
	if len(raw) == 0 {
		return nil
	}
	return raw
}

func randomID() string {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		return fmt.Sprintf("%d", time.Now().UnixNano())
	}
	return hex.EncodeToString(b[:])
}
