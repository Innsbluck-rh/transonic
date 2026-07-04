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

	TypePlaybackCommandRequest = "playback.command.request"
	TypePlaybackReport         = "playback.report"

	TypePlaybackSharedSnapshot = "playback.shared.snapshot"
	TypePlaybackSharedUpdated  = "playback.shared.updated"
	TypePlaybackApply          = "playback.apply"
	TypePlaybackCommandError   = "playback.command.error"
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

	mu      sync.Mutex
	clients map[string]map[string]*Client
}

// The shared playback state (ConnectPlaybackState) and command payload
// (ConnectPlaybackCommand) types are generated from the Rust wire types into
// connect_types_gen.go. Do not hand-define them here; regenerate with
// `pnpm go-types:export`. See docs/known-issues.md Tier 1.

func NewHub(store *store.Store, presenceTimeout time.Duration) *Hub {
	return &Hub{
		store:           store,
		presenceTimeout: presenceTimeout,
		clients:         make(map[string]map[string]*Client),
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
	shared, err := h.sharedOrDefault(ctx, session.UserKey)
	if err == nil {
		h.send(client, TypePlaybackSharedSnapshot, "", sharedPayload(shared, "snapshot"))
	}
	return client, nil
}

func (h *Hub) Unregister(ctx context.Context, client *Client) {
	removed := false
	h.mu.Lock()
	if byDevice := h.clients[client.Session.UserKey]; byDevice != nil {
		if byDevice[client.Session.DeviceID] == client {
			delete(byDevice, client.Session.DeviceID)
			removed = true
			if len(byDevice) == 0 {
				delete(h.clients, client.Session.UserKey)
			}
		}
	}
	close(client.Send)
	h.mu.Unlock()

	if !removed {
		return
	}
	h.freezeActiveDeviceIfOffline(ctx, client.Session)
	h.broadcastPresence(ctx, client.Session.UserKey)
}

func (h *Hub) Handle(ctx context.Context, client *Client, inbound Envelope) {
	if inbound.ProtocolVersion != 0 && inbound.ProtocolVersion != version.ProtocolVersion {
		h.sendCommandError(client, inbound.ID, "", "protocol version is not supported")
		return
	}

	switch inbound.Type {
	case TypePresenceHello, TypePresenceHeartbeat:
		_ = h.touchDevice(ctx, client.Session)
		h.broadcastPresence(ctx, client.Session.UserKey)
	case TypePlaybackCommandRequest:
		h.handlePlaybackCommandRequest(ctx, client, inbound)
	case TypePlaybackReport:
		h.handlePlaybackReport(ctx, client, inbound)
	default:
		h.sendCommandError(client, inbound.ID, "", fmt.Sprintf("unsupported message type %q", inbound.Type))
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

func (h *Hub) handlePlaybackCommandRequest(ctx context.Context, client *Client, inbound Envelope) {
	var command ConnectPlaybackCommand
	if err := json.Unmarshal(inbound.Payload, &command); err != nil || command.Op == "" {
		h.sendCommandError(client, inbound.ID, "", "playback.command.request requires op")
		return
	}
	commandID := ""
	if command.CommandID != nil {
		commandID = *command.CommandID
	}
	if commandID == "" {
		commandID = inbound.ID
	}
	if commandID == "" {
		commandID = randomID()
	}

	current, err := h.sharedOrDefault(ctx, client.Session.UserKey)
	if err != nil {
		h.sendCommandError(client, inbound.ID, commandID, "failed to load shared playback state")
		return
	}
	if command.BaseSeq != nil && *command.BaseSeq != current.Seq {
		h.sendCommandError(client, inbound.ID, commandID, "shared playback state is stale")
		h.send(client, TypePlaybackSharedSnapshot, inbound.ID, sharedPayload(current, "stale"))
		return
	}

	switch command.Op {
	case "setQueue", "insertAfterCurrent", "appendToQueue", "moveQueueIndex", "removeQueueIndex", "clearQueue":
		next, err := reduceQueueCommand(current, command, client.Session.DeviceID)
		if err != nil {
			h.sendCommandError(client, inbound.ID, commandID, err.Error())
			return
		}
		if err := h.saveAndBroadcast(ctx, client.Session.UserKey, next, client.Session.DeviceID, "command"); err != nil {
			h.sendCommandError(client, inbound.ID, commandID, "failed to save shared playback state")
			return
		}
		if command.Op == "setQueue" && command.AutoPlay != nil && *command.AutoPlay {
			h.applyToActiveDevice(client, inbound.ID, commandID, next.ActiveDeviceID, "play", nil)
		}
	case "transferPlayback":
		next, err := h.reduceTransferCommand(ctx, client, current, command)
		if err != nil {
			h.sendCommandError(client, inbound.ID, commandID, err.Error())
			return
		}
		if err := h.saveAndBroadcast(ctx, client.Session.UserKey, next, client.Session.DeviceID, "transfer"); err != nil {
			h.sendCommandError(client, inbound.ID, commandID, "failed to save shared playback state")
		}
	case "playQueueIndex":
		next, err := reducePlayQueueIndexCommand(current, command, client.Session.DeviceID)
		if err != nil {
			h.sendCommandError(client, inbound.ID, commandID, err.Error())
			return
		}
		if err := h.saveAndBroadcast(ctx, client.Session.UserKey, next, client.Session.DeviceID, "command"); err != nil {
			h.sendCommandError(client, inbound.ID, commandID, "failed to save shared playback state")
			return
		}
		h.applyToActiveDevice(client, inbound.ID, commandID, next.ActiveDeviceID, "play", nil)
	case "play", "pause", "stop", "seek", "next", "prev":
		next := *current
		if next.ActiveDeviceID == "" {
			next.ActiveDeviceID = client.Session.DeviceID
			next.Seq++
			next.UpdatedAt = time.Now().UTC()
			next.UpdatedByDeviceID = client.Session.DeviceID
			if err := h.store.SaveSharedPlaybackState(ctx, next); err != nil {
				h.sendCommandError(client, inbound.ID, commandID, "failed to save active playback device")
				return
			}
			h.broadcast(client.Session.UserKey, TypePlaybackSharedUpdated, inbound.ID, sharedPayload(&next, "command"))
		}
		args := playbackApplyArgs(command)
		h.applyToActiveDevice(client, inbound.ID, commandID, next.ActiveDeviceID, command.Op, args)
	default:
		h.sendCommandError(client, inbound.ID, commandID, fmt.Sprintf("unsupported playback op %q", command.Op))
	}
}

func (h *Hub) handlePlaybackReport(ctx context.Context, client *Client, inbound Envelope) {
	var report struct {
		CommandID string          `json:"commandId,omitempty"`
		BaseSeq   *int64          `json:"baseSeq,omitempty"`
		State     json.RawMessage `json:"state"`
	}
	if err := json.Unmarshal(inbound.Payload, &report); err != nil || len(report.State) == 0 {
		h.sendCommandError(client, inbound.ID, report.CommandID, "playback.report requires state")
		return
	}

	current, err := h.sharedOrDefault(ctx, client.Session.UserKey)
	if err != nil {
		h.sendCommandError(client, inbound.ID, report.CommandID, "failed to load shared playback state")
		return
	}
	if current.ActiveDeviceID != "" && current.ActiveDeviceID != client.Session.DeviceID {
		h.sendCommandError(client, inbound.ID, report.CommandID, "only the active device can report playback state")
		return
	}

	currentState, err := decodePlaybackState(current.State)
	if err != nil {
		h.sendCommandError(client, inbound.ID, report.CommandID, err.Error())
		return
	}
	reportState, err := decodePlaybackState(report.State)
	if err != nil {
		h.sendCommandError(client, inbound.ID, report.CommandID, err.Error())
		return
	}
	nextState, changed := mergePlaybackReportState(currentState, reportState)
	if !changed {
		return
	}
	raw, err := encodePlaybackState(nextState)
	if err != nil {
		h.sendCommandError(client, inbound.ID, report.CommandID, err.Error())
		return
	}
	next := store.SharedPlaybackState{
		UserKey:           client.Session.UserKey,
		Seq:               current.Seq,
		ActiveDeviceID:    current.ActiveDeviceID,
		State:             raw,
		UpdatedAt:         time.Now().UTC(),
		UpdatedByDeviceID: client.Session.DeviceID,
	}
	if next.ActiveDeviceID == "" {
		next.ActiveDeviceID = client.Session.DeviceID
	}
	if err := h.saveAndBroadcast(ctx, client.Session.UserKey, &next, client.Session.DeviceID, "report"); err != nil {
		h.sendCommandError(client, inbound.ID, report.CommandID, "failed to save shared playback state")
	}
}

func mergePlaybackReportState(currentState ConnectPlaybackState, reportState ConnectPlaybackState) (ConnectPlaybackState, bool) {
	reportIndex := compatibleReportIndex(currentState.Queue, reportState.CurrentIndex, reportState.CurrentSongID)
	if reportIndex == nil {
		return currentState, false
	}
	next := currentState
	previousCurrentIndex := cloneInt64Ptr(next.CurrentIndex)
	previousPlayNextQueueLen := next.PlayNextQueueLen
	next.PlayingState = reportState.PlayingState
	next.CurrentIndex = reportIndex
	next.PlayNextQueueLen = adjustedPlayNextQueueLenForCurrentChange(
		len(next.Queue),
		previousCurrentIndex,
		previousPlayNextQueueLen,
		reportIndex,
	)
	next.CurrentSongID = songIDAt(next.Queue, *reportIndex)
	next.CurrentPositionMs = reportState.CurrentPositionMs
	return next, true
}

func compatibleReportIndex(queue []json.RawMessage, reportIndex *int64, reportSongID *string) *int64 {
	if len(queue) == 0 {
		return nil
	}
	if reportIndex != nil && *reportIndex >= 0 && *reportIndex < int64(len(queue)) {
		expectedSongID := songIDAt(queue, *reportIndex)
		if expectedSongID == nil || (reportSongID != nil && *reportSongID == *expectedSongID) {
			index := *reportIndex
			return &index
		}
	}
	if reportSongID == nil {
		return nil
	}
	for index := range queue {
		expectedSongID := songIDAt(queue, int64(index))
		if expectedSongID != nil && *expectedSongID == *reportSongID {
			compatibleIndex := int64(index)
			return &compatibleIndex
		}
	}
	return nil
}

func (h *Hub) reduceTransferCommand(ctx context.Context, client *Client, current *store.SharedPlaybackState, command ConnectPlaybackCommand) (*store.SharedPlaybackState, error) {
	targetDeviceID := ""
	if command.TargetDeviceID != nil {
		targetDeviceID = *command.TargetDeviceID
	}
	if targetDeviceID == "" {
		return nil, errors.New("transferPlayback requires targetDeviceId")
	}
	if !h.isDeviceOnline(client.Session.UserKey, targetDeviceID) {
		return nil, errors.New("target device is offline")
	}
	state, err := decodePlaybackState(current.State)
	if err != nil {
		return nil, err
	}
	if current.ActiveDeviceID == "" {
		current.ActiveDeviceID = client.Session.DeviceID
	} else if current.ActiveDeviceID == targetDeviceID {
		return nil, errors.New("target device is already active")
	}
	if len(state.Queue) == 0 {
		return nil, errors.New("playback queue is empty")
	}
	state.CurrentPositionMs = estimatedPosition(current, state)
	raw, err := encodePlaybackState(state)
	if err != nil {
		return nil, err
	}
	return &store.SharedPlaybackState{
		UserKey:           client.Session.UserKey,
		Seq:               current.Seq + 1,
		ActiveDeviceID:    targetDeviceID,
		State:             raw,
		UpdatedAt:         time.Now().UTC(),
		UpdatedByDeviceID: client.Session.DeviceID,
	}, nil
}

func reduceQueueCommand(current *store.SharedPlaybackState, command ConnectPlaybackCommand, fallbackActiveDeviceID string) (*store.SharedPlaybackState, error) {
	state, err := decodePlaybackState(current.State)
	if err != nil {
		return nil, err
	}
	switch command.Op {
	case "setQueue":
		state.Queue = cloneRawMessages(command.Queue)
		state.PlayNextQueueLen = 0
		if len(state.Queue) == 0 {
			state.CurrentIndex = nil
		} else if command.CurrentIndex != nil {
			state.CurrentIndex = command.CurrentIndex
		} else {
			zero := int64(0)
			state.CurrentIndex = &zero
		}
		state.CurrentPositionMs = 0
		state.PlayingState = stoppedOrIdle(state.Queue)
	case "insertAfterCurrent":
		if len(command.Items) == 0 {
			return current, nil
		}
		insertAt := playNextEndIndex(state)
		nextQueue := make([]json.RawMessage, 0, len(state.Queue)+len(command.Items))
		nextQueue = append(nextQueue, state.Queue[:insertAt]...)
		nextQueue = append(nextQueue, cloneRawMessages(command.Items)...)
		nextQueue = append(nextQueue, state.Queue[insertAt:]...)
		state.Queue = nextQueue
		state.PlayNextQueueLen += int64(len(command.Items))
		if state.CurrentIndex == nil {
			zero := int64(0)
			state.CurrentIndex = &zero
			state.CurrentPositionMs = 0
			state.PlayingState = stoppedOrIdle(state.Queue)
		}
		clampPlayNextQueueLen(&state)
	case "appendToQueue":
		if len(command.Items) == 0 {
			return current, nil
		}
		state.Queue = append(state.Queue, cloneRawMessages(command.Items)...)
		if state.CurrentIndex == nil {
			zero := int64(0)
			state.CurrentIndex = &zero
			state.PlayNextQueueLen = 0
			state.CurrentPositionMs = 0
			state.PlayingState = stoppedOrIdle(state.Queue)
		}
	case "moveQueueIndex":
		if command.FromIndex == nil || command.ToIndex == nil {
			return nil, errors.New("moveQueueIndex requires fromIndex and toIndex")
		}
		if err := moveQueueIndex(&state, *command.FromIndex, *command.ToIndex); err != nil {
			return nil, err
		}
	case "removeQueueIndex":
		if command.Index == nil {
			return nil, errors.New("removeQueueIndex requires index")
		}
		if err := removeQueueIndex(&state, *command.Index); err != nil {
			return nil, err
		}
	case "clearQueue":
		state = emptyPlaybackState()
	}
	if err := normalizePlaybackState(&state); err != nil {
		return nil, err
	}
	raw, err := encodePlaybackState(state)
	if err != nil {
		return nil, err
	}
	activeDeviceID := current.ActiveDeviceID
	if activeDeviceID == "" && len(state.Queue) > 0 {
		activeDeviceID = fallbackActiveDeviceID
	}
	return &store.SharedPlaybackState{
		UserKey:           current.UserKey,
		Seq:               current.Seq + 1,
		ActiveDeviceID:    activeDeviceID,
		State:             raw,
		UpdatedAt:         time.Now().UTC(),
		UpdatedByDeviceID: fallbackActiveDeviceID,
	}, nil
}

func reducePlayQueueIndexCommand(current *store.SharedPlaybackState, command ConnectPlaybackCommand, fallbackActiveDeviceID string) (*store.SharedPlaybackState, error) {
	if command.Index == nil {
		return nil, errors.New("playQueueIndex requires index")
	}
	state, err := decodePlaybackState(current.State)
	if err != nil {
		return nil, err
	}
	index := *command.Index
	if index < 0 || index >= int64(len(state.Queue)) {
		return nil, errors.New("playback queue index is out of range")
	}
	previousCurrentIndex := cloneInt64Ptr(state.CurrentIndex)
	previousPlayNextQueueLen := state.PlayNextQueueLen
	state.CurrentIndex = &index
	state.PlayNextQueueLen = adjustedPlayNextQueueLenForCurrentChange(
		len(state.Queue),
		previousCurrentIndex,
		previousPlayNextQueueLen,
		&index,
	)
	state.CurrentPositionMs = 0
	state.PlayingState = stoppedOrIdle(state.Queue)
	if err := normalizePlaybackState(&state); err != nil {
		return nil, err
	}
	raw, err := encodePlaybackState(state)
	if err != nil {
		return nil, err
	}
	activeDeviceID := current.ActiveDeviceID
	if activeDeviceID == "" {
		activeDeviceID = fallbackActiveDeviceID
	}
	return &store.SharedPlaybackState{
		UserKey:           current.UserKey,
		Seq:               current.Seq + 1,
		ActiveDeviceID:    activeDeviceID,
		State:             raw,
		UpdatedAt:         time.Now().UTC(),
		UpdatedByDeviceID: fallbackActiveDeviceID,
	}, nil
}

func moveQueueIndex(state *ConnectPlaybackState, fromIndex, toIndex int64) error {
	if fromIndex < 0 || toIndex < 0 || fromIndex >= int64(len(state.Queue)) || toIndex >= int64(len(state.Queue)) {
		return errors.New("playback queue index is out of range")
	}
	if fromIndex == toIndex {
		return nil
	}
	previousCurrentIndex := cloneInt64Ptr(state.CurrentIndex)
	previousPlayNextQueueLen := state.PlayNextQueueLen
	entry := state.Queue[fromIndex]
	state.Queue = append(state.Queue[:fromIndex], state.Queue[fromIndex+1:]...)
	state.Queue = append(state.Queue[:toIndex], append([]json.RawMessage{entry}, state.Queue[toIndex:]...)...)

	if state.CurrentIndex != nil {
		current := *state.CurrentIndex
		switch {
		case current == fromIndex:
			current = toIndex
		case fromIndex < current && toIndex >= current:
			current--
		case fromIndex > current && toIndex <= current:
			current++
		}
		state.CurrentIndex = &current
	}
	if int64PtrEqual(previousCurrentIndex, state.CurrentIndex) {
		state.PlayNextQueueLen = clampedPlayNextQueueLen(len(state.Queue), state.CurrentIndex, previousPlayNextQueueLen)
	} else {
		state.PlayNextQueueLen = adjustedPlayNextQueueLenForCurrentChange(
			len(state.Queue),
			previousCurrentIndex,
			previousPlayNextQueueLen,
			state.CurrentIndex,
		)
	}
	return nil
}

func removeQueueIndex(state *ConnectPlaybackState, index int64) error {
	if index < 0 || index >= int64(len(state.Queue)) {
		return errors.New("playback queue index is out of range")
	}
	previousCurrentIndex := cloneInt64Ptr(state.CurrentIndex)
	previousPlayNextQueueLen := state.PlayNextQueueLen
	removingCurrent := state.CurrentIndex != nil && *state.CurrentIndex == index
	wasInPlayNextQueue := isPlayNextQueueIndex(len(state.Queue), previousCurrentIndex, previousPlayNextQueueLen, index)
	state.Queue = append(state.Queue[:index], state.Queue[index+1:]...)
	if len(state.Queue) == 0 {
		state.CurrentIndex = nil
		state.PlayNextQueueLen = 0
		state.CurrentPositionMs = 0
		state.PlayingState = "idle"
		return nil
	}
	if state.CurrentIndex != nil {
		current := *state.CurrentIndex
		switch {
		case current > index:
			current--
		case current == index && current >= int64(len(state.Queue)):
			current = int64(len(state.Queue) - 1)
		}
		state.CurrentIndex = &current
		if current == index {
			state.CurrentPositionMs = 0
		}
	}
	nextPlayNextQueueLen := previousPlayNextQueueLen
	if removingCurrent && nextPlayNextQueueLen > 0 {
		nextPlayNextQueueLen--
	} else if wasInPlayNextQueue && nextPlayNextQueueLen > 0 {
		nextPlayNextQueueLen--
	}
	state.PlayNextQueueLen = clampedPlayNextQueueLen(len(state.Queue), state.CurrentIndex, nextPlayNextQueueLen)
	return nil
}

func (h *Hub) saveAndBroadcast(ctx context.Context, userKey string, state *store.SharedPlaybackState, updatedByDeviceID, reason string) error {
	if state.UserKey == "" {
		state.UserKey = userKey
	}
	if state.UpdatedAt.IsZero() {
		state.UpdatedAt = time.Now().UTC()
	}
	if state.UpdatedByDeviceID == "" {
		state.UpdatedByDeviceID = updatedByDeviceID
	}
	if err := h.store.SaveSharedPlaybackState(ctx, *state); err != nil {
		return err
	}
	h.broadcast(userKey, TypePlaybackSharedUpdated, "", sharedPayload(state, reason))
	return nil
}

func (h *Hub) applyToActiveDevice(client *Client, replyTo, commandID, activeDeviceID, op string, args map[string]any) {
	if activeDeviceID == "" {
		h.sendCommandError(client, replyTo, commandID, "active device is unavailable")
		return
	}
	if !h.sendToDevice(client.Session.UserKey, activeDeviceID, TypePlaybackApply, replyTo, mustJSON(map[string]any{
		"commandId": commandID,
		"op":        op,
		"args":      args,
	})) {
		h.sendCommandError(client, replyTo, commandID, "active device is offline")
	}
}

func playbackApplyArgs(command ConnectPlaybackCommand) map[string]any {
	args := map[string]any{}
	if command.PositionMs != nil {
		args["positionMs"] = *command.PositionMs
	}
	if command.Index != nil {
		args["index"] = *command.Index
	}
	return args
}

func (h *Hub) freezeActiveDeviceIfOffline(ctx context.Context, session store.Session) {
	if h.isDeviceOnline(session.UserKey, session.DeviceID) {
		return
	}
	current, err := h.sharedOrDefault(ctx, session.UserKey)
	if err != nil || current.ActiveDeviceID != session.DeviceID {
		return
	}
	state, err := decodePlaybackState(current.State)
	if err != nil {
		return
	}
	if state.PlayingState == "playing" {
		state.CurrentPositionMs = estimatedPosition(current, state)
	}
	state.PlayingState = stoppedOrIdle(state.Queue)
	raw, err := encodePlaybackState(state)
	if err != nil {
		return
	}
	next := store.SharedPlaybackState{
		UserKey:           session.UserKey,
		Seq:               current.Seq + 1,
		ActiveDeviceID:    "",
		State:             raw,
		UpdatedAt:         time.Now().UTC(),
		UpdatedByDeviceID: session.DeviceID,
	}
	_ = h.saveAndBroadcast(ctx, session.UserKey, &next, session.DeviceID, "activeOffline")
}

func (h *Hub) sharedOrDefault(ctx context.Context, userKey string) (*store.SharedPlaybackState, error) {
	current, err := h.store.GetSharedPlaybackState(ctx, userKey)
	if err != nil {
		return nil, err
	}
	if current != nil {
		return current, nil
	}
	state, err := encodePlaybackState(emptyPlaybackState())
	if err != nil {
		return nil, err
	}
	return &store.SharedPlaybackState{
		UserKey:   userKey,
		Seq:       0,
		State:     state,
		UpdatedAt: time.Now().UTC(),
	}, nil
}

func decodePlaybackState(raw json.RawMessage) (ConnectPlaybackState, error) {
	var state ConnectPlaybackState
	if len(raw) == 0 {
		return emptyPlaybackState(), nil
	}
	if err := json.Unmarshal(raw, &state); err != nil {
		return ConnectPlaybackState{}, errors.New("state must be a JSON object compatible with PlaybackStatus")
	}
	if state.PlayingState == "" {
		state.PlayingState = "idle"
	}
	if state.Queue == nil {
		state.Queue = []json.RawMessage{}
	}
	if err := normalizePlaybackState(&state); err != nil {
		return ConnectPlaybackState{}, err
	}
	return state, nil
}

func normalizePlaybackState(state *ConnectPlaybackState) error {
	if state.CurrentPositionMs < 0 {
		return errors.New("state.currentPositionMs must not be negative")
	}
	if state.PlayNextQueueLen < 0 {
		return errors.New("state.playNextQueueLen must not be negative")
	}
	if len(state.Queue) == 0 {
		state.CurrentIndex = nil
		state.CurrentSongID = nil
		state.PlayNextQueueLen = 0
		state.CurrentPositionMs = 0
		if state.PlayingState == "" || state.PlayingState == "stopped" || state.PlayingState == "paused" || state.PlayingState == "playing" {
			state.PlayingState = "idle"
		}
		return nil
	}
	if state.CurrentIndex != nil {
		if *state.CurrentIndex < 0 || *state.CurrentIndex >= int64(len(state.Queue)) {
			return errors.New("state.currentIndex is out of range")
		}
		songID := songIDAt(state.Queue, *state.CurrentIndex)
		state.CurrentSongID = songID
	} else {
		state.CurrentSongID = nil
	}
	clampPlayNextQueueLen(state)
	if state.PlayingState == "" || state.PlayingState == "idle" {
		state.PlayingState = "stopped"
	}
	return nil
}

func encodePlaybackState(state ConnectPlaybackState) (json.RawMessage, error) {
	if state.Queue == nil {
		state.Queue = []json.RawMessage{}
	}
	if err := normalizePlaybackState(&state); err != nil {
		return nil, err
	}
	raw, err := json.Marshal(state)
	return json.RawMessage(raw), err
}

func emptyPlaybackState() ConnectPlaybackState {
	return ConnectPlaybackState{
		PlayingState:      "idle",
		Queue:             []json.RawMessage{},
		CurrentPositionMs: 0,
	}
}

func stoppedOrIdle(queue []json.RawMessage) PlayingState {
	if len(queue) == 0 {
		return "idle"
	}
	return "stopped"
}

func songIDAt(queue []json.RawMessage, index int64) *string {
	if index < 0 || index >= int64(len(queue)) {
		return nil
	}
	var entry struct {
		ID string `json:"id"`
	}
	if err := json.Unmarshal(queue[index], &entry); err != nil || entry.ID == "" {
		return nil
	}
	return &entry.ID
}

// Play-next region math. This reducer is intentionally separate from the Rust
// local-playback reducer; the one invariant both must preserve (what
// playNextQueueLen means) is documented in docs/playback-queue-semantics.md.
func playNextStartIndex(queueLen int, currentIndex *int64) int {
	if currentIndex == nil {
		return 0
	}
	start := *currentIndex + 1
	if start < 0 {
		return 0
	}
	if start > int64(queueLen) {
		return queueLen
	}
	return int(start)
}

func clampedPlayNextQueueLen(queueLen int, currentIndex *int64, playNextQueueLen int64) int64 {
	if playNextQueueLen <= 0 {
		return 0
	}
	start := playNextStartIndex(queueLen, currentIndex)
	maxLen := int64(queueLen - start)
	if playNextQueueLen > maxLen {
		return maxLen
	}
	return playNextQueueLen
}

func playNextEndIndex(state ConnectPlaybackState) int {
	start := playNextStartIndex(len(state.Queue), state.CurrentIndex)
	end := start + int(clampedPlayNextQueueLen(len(state.Queue), state.CurrentIndex, state.PlayNextQueueLen))
	if end > len(state.Queue) {
		return len(state.Queue)
	}
	return end
}

func clampPlayNextQueueLen(state *ConnectPlaybackState) {
	state.PlayNextQueueLen = clampedPlayNextQueueLen(len(state.Queue), state.CurrentIndex, state.PlayNextQueueLen)
}

func isPlayNextQueueIndex(queueLen int, currentIndex *int64, playNextQueueLen int64, queueIndex int64) bool {
	if queueIndex < 0 {
		return false
	}
	start := int64(playNextStartIndex(queueLen, currentIndex))
	end := start + clampedPlayNextQueueLen(queueLen, currentIndex, playNextQueueLen)
	return queueIndex >= start && queueIndex < end
}

func adjustedPlayNextQueueLenForCurrentChange(queueLen int, previousCurrentIndex *int64, previousPlayNextQueueLen int64, nextCurrentIndex *int64) int64 {
	if previousPlayNextQueueLen <= 0 {
		return 0
	}
	if int64PtrEqual(previousCurrentIndex, nextCurrentIndex) {
		return clampedPlayNextQueueLen(queueLen, nextCurrentIndex, previousPlayNextQueueLen)
	}
	previousStart := int64(playNextStartIndex(queueLen, previousCurrentIndex))
	previousEnd := previousStart + clampedPlayNextQueueLen(queueLen, previousCurrentIndex, previousPlayNextQueueLen)
	if nextCurrentIndex == nil || *nextCurrentIndex < previousStart || *nextCurrentIndex >= previousEnd {
		return 0
	}
	return previousEnd - *nextCurrentIndex - 1
}

func cloneInt64Ptr(value *int64) *int64 {
	if value == nil {
		return nil
	}
	cloned := *value
	return &cloned
}

func int64PtrEqual(a *int64, b *int64) bool {
	if a == nil || b == nil {
		return a == b
	}
	return *a == *b
}

func cloneRawMessages(items []json.RawMessage) []json.RawMessage {
	if len(items) == 0 {
		return []json.RawMessage{}
	}
	clone := make([]json.RawMessage, len(items))
	for i := range items {
		clone[i] = append(json.RawMessage(nil), items[i]...)
	}
	return clone
}

func sharedPayload(shared *store.SharedPlaybackState, reason string) json.RawMessage {
	if shared == nil {
		return mustJSON(map[string]any{
			"seq":            0,
			"activeDeviceId": nil,
			"state":          emptyPlaybackState(),
			"updateReason":   reason,
		})
	}
	state, err := decodePlaybackState(shared.State)
	if err == nil {
		state.CurrentPositionMs = estimatedPosition(shared, state)
	}
	var stateValue any = json.RawMessage(shared.State)
	if err == nil {
		stateValue = state
	}
	var activeDeviceID any
	if shared.ActiveDeviceID != "" {
		activeDeviceID = shared.ActiveDeviceID
	}
	var updatedByDeviceID any
	if shared.UpdatedByDeviceID != "" {
		updatedByDeviceID = shared.UpdatedByDeviceID
	}
	return mustJSON(map[string]any{
		"seq":               shared.Seq,
		"activeDeviceId":    activeDeviceID,
		"state":             stateValue,
		"updatedAt":         shared.UpdatedAt,
		"updatedByDeviceId": updatedByDeviceID,
		"updateReason":      reason,
	})
}

func estimatedPosition(shared *store.SharedPlaybackState, state ConnectPlaybackState) int64 {
	if state.PlayingState != "playing" {
		return state.CurrentPositionMs
	}
	elapsed := time.Since(shared.UpdatedAt).Milliseconds()
	if elapsed < 0 {
		elapsed = 0
	}
	return state.CurrentPositionMs + elapsed
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

func (h *Hub) isDeviceOnline(userKey, deviceID string) bool {
	h.mu.Lock()
	defer h.mu.Unlock()
	return h.clients[userKey][deviceID] != nil
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

func (h *Hub) sendCommandError(client *Client, replyTo, commandID, message string) {
	h.send(client, TypePlaybackCommandError, replyTo, mustJSON(map[string]any{
		"commandId": commandID,
		"message":   message,
	}))
}

func mustJSON(value any) json.RawMessage {
	raw, err := json.Marshal(value)
	if err != nil {
		return json.RawMessage(`{"error":"json marshal failed"}`)
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
