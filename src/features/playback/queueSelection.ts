/**
 * Multi-selection of queue rows. Rows are identified by queue index; the index
 * map each queue mutation carries (see docs/playback-queue-semantics.md) moves
 * the selection along with the entries.
 */
export interface QueueSelection {
  /** Selected queue indices, ascending. Empty means the queue is not in selection mode. */
  indices: number[];
  /** Fixed end of a Shift+click range. */
  anchor: number | null;
}

export const emptyQueueSelection: QueueSelection = { indices: [], anchor: null };

function selection(indices: Iterable<number>, anchor: number | null): QueueSelection {
  const sorted = [...new Set(indices)].sort((a, b) => a - b);
  return sorted.length ? { indices: sorted, anchor } : emptyQueueSelection;
}

function span(from: number, to: number) {
  const start = Math.min(from, to);
  return Array.from({ length: Math.abs(to - from) + 1 }, (_, offset) => start + offset);
}

/** Ctrl+click or a click on the move handle: flips one row and makes it the anchor. */
export function toggleQueueIndex(current: QueueSelection, index: number): QueueSelection {
  const indices = current.indices.includes(index) ? current.indices.filter((value) => value !== index) : [...current.indices, index];
  return selection(indices, index);
}

/** Adds a row without moving the anchor, e.g. a drag started from an unselected row. */
export function addQueueIndex(current: QueueSelection, index: number): QueueSelection {
  return selection([...current.indices, index], current.anchor ?? index);
}

/** Shift+click: selects only anchor..index. Without a selection the row becomes the anchor. */
export function selectQueueRange(current: QueueSelection, index: number): QueueSelection {
  if (current.anchor === null || !current.indices.length) {
    return selection([index], index);
  }
  return selection(span(current.anchor, index), current.anchor);
}

/**
 * Rectangle selection: the rows from..to (null: no row covered) added to `base`, the
 * selection kept from before the rectangle (empty when the rectangle replaces it).
 */
export function selectQueueSpan(rows: { from: number; to: number } | null, anchor: number, base: readonly number[] = []): QueueSelection {
  return selection([...base, ...(rows ? span(rows.from, rows.to) : [])], anchor);
}

export function selectAllQueue(length: number): QueueSelection {
  return length > 0 ? selection(span(0, length - 1), 0) : emptyQueueSelection;
}

/** Follows a queue mutation: `indexMap[old]` is the new index, -1 when the entry was removed. */
export function remapQueueSelection(current: QueueSelection, indexMap: readonly number[]): QueueSelection {
  const moved = (index: number | null) => (index === null ? -1 : (indexMap[index] ?? -1));
  const anchor = moved(current.anchor);
  return selection(
    current.indices.map(moved).filter((index) => index >= 0),
    anchor >= 0 ? anchor : null
  );
}

/**
 * Whether moving `indices` into the gap before `insertBefore` changes nothing:
 * the selection is one contiguous block and the gap is inside or beside it.
 */
export function isQueueMoveNoop(indices: readonly number[], insertBefore: number) {
  if (!indices.length) {
    return true;
  }
  const first = indices[0];
  const last = indices[indices.length - 1];
  return last - first + 1 === indices.length && insertBefore >= first && insertBefore <= last + 1;
}
