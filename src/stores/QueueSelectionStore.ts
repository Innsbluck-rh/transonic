import { createEffect, createMemo, createRoot, createSignal, on } from 'solid-js';
import { commands } from '~/bindings';
import {
  addQueueIndex,
  emptyQueueSelection,
  remapQueueSelection,
  selectAllQueue,
  selectQueueRange,
  selectQueueSpan,
  toggleQueueIndex,
  type QueueSelection,
} from '~/features/playback/queueSelection';
import { hasPlaybackCommandError } from '~/features/playback/service';
import { usePlaybackStatus } from '~/features/playback/usePlaybackStatus';

// Module level, not component state: on SP the queue list unmounts whenever the
// player sheet closes, and the selection outlives that.
const [queueSelection, setQueueSelection] = createSignal<QueueSelection>(emptyQueueSelection);

const queueStatus = createRoot(() => {
  const status = usePlaybackStatus();
  const queueLength = createMemo(() => status()?.queue.length ?? 0);
  const queueRevision = createMemo(() => status()?.queueRevision ?? 0);
  const queueKey = createMemo(() => (status()?.queue ?? []).map((entry) => entry.id).join('\0'));

  // A mutation one revision past the last one seen carries the map that moves
  // the selection; anything else (a skipped revision, a queue replaced without
  // one) leaves no way to tell where the rows went, so the selection ends.
  let previous = { revision: queueRevision(), queueKey: queueKey(), length: queueLength() };
  createEffect(
    on(
      [queueRevision, queueKey],
      ([revision, key]) => {
        const last = previous;
        previous = { revision, queueKey: key, length: queueLength() };
        if (!queueSelection().indices.length || (revision === last.revision && key === last.queueKey)) {
          return;
        }
        const indexMap = status()?.queueIndexMap ?? [];
        if (revision === last.revision + 1 && indexMap.length === last.length) {
          setQueueSelection((current) => remapQueueSelection(current, indexMap));
        } else {
          setQueueSelection(emptyQueueSelection);
        }
      },
      { defer: true }
    )
  );

  return { queueLength };
});

export { queueSelection };

export const isQueueSelecting = () => queueSelection().indices.length > 0;

export const isQueueIndexSelected = (index: number) => queueSelection().indices.includes(index);

export const toggleQueueSelection = (index: number) => setQueueSelection((current) => toggleQueueIndex(current, index));

export const addToQueueSelection = (index: number) => setQueueSelection((current) => addQueueIndex(current, index));

export const selectQueueRangeTo = (index: number) => setQueueSelection((current) => selectQueueRange(current, index));

export const setQueueSelectionSpan = (rows: { from: number; to: number } | null, anchor: number, base?: readonly number[]) =>
  setQueueSelection(selectQueueSpan(rows, anchor, base));

export const selectAllQueueRows = () => setQueueSelection(selectAllQueue(queueStatus.queueLength()));

export const clearQueueSelection = () => setQueueSelection(emptyQueueSelection);

/** Remove from Queue: removing the rows ends the selection. */
export const removeSelectedQueueRows = (indices = queueSelection().indices) => {
  clearQueueSelection();
  if (indices.length) {
    commands.playbackRemoveQueueIndices({ indices }).then(hasPlaybackCommandError);
  }
};
