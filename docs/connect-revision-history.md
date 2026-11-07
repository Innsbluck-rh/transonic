# Connect protocol revision history

Changes to the app ⟷ Connect server protocol, per `protocolVersion`. The
version goes up with any change after which the current app and the previous
server (or the other way round) can no longer communicate correctly. Versions
up to 2 are not recorded.

## 3

`playback.command.request`:

- Removed op `moveQueueIndex` and its fields `fromIndex` / `toIndex`.
- Added op `moveQueueIndices` (`indices`, `insertBefore`): moves the entries at
  `indices`, kept in queue order, into the gap before `insertBefore` (an index
  into the current queue; the queue length means the end).
- Added op `removeQueueIndices` (`indices`): removes the entries at `indices`.
  When the current entry is removed, the first surviving entry after it becomes
  current at position 0, `paused` if the removed entry was loaded (`playing`,
  `paused` or `interrupted`) and `stopped` otherwise; when no entry survives
  after it, the last surviving entry becomes current and the state is
  `stopped`.

Shared playback state:

- Added `queueRevision` and `queueIndexMap` (absent: `0` and empty), following
  the rule in `playback-queue-semantics.md` ("Queue revision and index map").
- `pushState` does not take the sender's `queueRevision`: the shared revision
  advances by one and `queueIndexMap` becomes empty.
