# Playback queue semantics — the shared invariant

Local playback (Rust, `src-tauri/src/playback/controller.rs`) and Connect shared
playback (Go, `server/internal/realtime/hub.go`) each own an independent reducer
for queue operations. By design they are **not** unified: each serves a different
context (Connect off → Rust; Connect on → Go), and the two never run at once.

The wire **types** are unified — Rust is the single source of truth and the Go
structs are generated from it (`ConnectPlaybackState` / `ConnectPlaybackCommand`
in `connect_types_gen.go`; see `docs/known-issues.md` Tier 1). The reducer
**behaviour** is deliberately left separate.

There is exactly **one** invariant both reducers must preserve, because a single
frontend consumer (`isPlayNextQueueIndex` in `src/features/playback/usePlayback.ts`)
decodes `playNextQueueLen` from both producers with one interpretation:

> **`playNextQueueLen` counts the entries immediately after the current track
> that were explicitly enqueued via "play next" (`insertAfterCurrent`).**
>
> The play-next region is the half-open index range
> `[start, start + playNextQueueLen)` where `start = currentIndex + 1`
> (or `0` when there is no current index), clamped to the queue length.
>
> `playNextQueueLen` is **not** derivable from the queue contents — queue entries
> carry no "was play-next" marker — so it is standalone mutable state each
> operation must maintain. Advancing past a play-next entry (`next`,
> `playQueueIndex`) consumes it; removing one decrements the count; moving the
> current index recomputes the region.

Any new queue operation, in either reducer, must keep this region definition
consistent. The read-side math lives in three places that must agree:

- Rust: `play_next_start_index` / `is_play_next_queue_index` (controller.rs)
- Go: `playNextStartIndex` / `isPlayNextQueueIndex` (hub.go)
- TS (display only): `isPlayNextQueueIndex` (usePlayback.ts)
