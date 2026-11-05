import { createSignal, onCleanup } from 'solid-js';
import { isQueueMoveNoop } from '~/features/playback/queueSelection';
import { cancelRipple } from '~/utils/ripple';

/** Distance a pointer has to travel before a press becomes a drag or a rectangle. */
const DRAG_THRESHOLD_PX = 5;
/** SP long-press on the move handle: the empty checkbox shows, then the row gets selected. */
export const LONG_PRESS_CHECKBOX_MS = 300;
export const LONG_PRESS_SELECT_MS = 600;
const AUTO_SCROLL_EDGE_PX = 48;
const AUTO_SCROLL_MAX_STEP_PX = 16;

export type QueueDropEdge = 'top' | 'bottom';

export interface QueueDropIndicator {
  index: number;
  edge: QueueDropEdge;
}

export interface QueueRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

interface QueueListGestureOptions {
  root: () => HTMLElement | undefined;
  scrollContainer: () => HTMLElement | undefined;
  queueLength: () => number;
  /** Rows to move when a drag starts from `index`; may select the row first. */
  beginMove: (index: number) => number[];
  move: (indices: number[], insertBefore: number) => void;
  /** Rectangle selection; `rows` is null while the rectangle covers no row. */
  selectRect: (rows: { from: number; to: number } | null, anchor: number) => void;
  /** SP long-press on the move handle, once held long enough. */
  longPress: (index: number) => void;
}

function rowElements(root: HTMLElement) {
  return [...root.querySelectorAll<HTMLElement>('[data-queue-index]')];
}

export function queueRowIndexAt(target: EventTarget | null): number | null {
  const row = target instanceof Element ? target.closest<HTMLElement>('[data-queue-index]') : null;
  return row ? Number(row.dataset.queueIndex) : null;
}

export function isQueueHandleTarget(target: EventTarget | null) {
  return target instanceof Element && target.closest('[data-queue-handle]') !== null;
}

/**
 * Pointer gestures of the queue list that a click cannot express: dragging rows
 * (PC: from anywhere on a row, SP: from the move handle), the PC Shift+drag
 * rectangle and the SP long-press on the move handle. Clicks and taps stay with
 * the list's click handler; `consumeClick` tells it when a gesture used them up.
 */
export function createQueueListGestures(options: QueueListGestureOptions) {
  const [dropIndicator, setDropIndicator] = createSignal<QueueDropIndicator | null>(null);
  const [rect, setRect] = createSignal<QueueRect | null>(null);
  /** Row showing the empty checkbox while an SP long-press on its move handle is held. */
  const [longPressPreview, setLongPressPreview] = createSignal<number | null>(null);
  let suppressClick = false;
  let endGesture: (() => void) | undefined;

  const insertBeforeAt = (clientY: number) => {
    const root = options.root();
    if (!root) return null;
    const rows = rowElements(root);
    const index = rows.findIndex((row) => {
      const box = row.getBoundingClientRect();
      return clientY < box.top + box.height / 2;
    });
    return index < 0 ? rows.length : index;
  };

  const autoScroll = (clientY: number) => {
    const container = options.scrollContainer();
    if (!container) return;
    const box = container.getBoundingClientRect();
    const overTop = box.top + AUTO_SCROLL_EDGE_PX - clientY;
    const overBottom = clientY - (box.bottom - AUTO_SCROLL_EDGE_PX);
    const over = overTop > 0 ? -overTop : overBottom > 0 ? overBottom : 0;
    if (over !== 0) {
      container.scrollTop += Math.sign(over) * Math.ceil(Math.min(Math.abs(over) / AUTO_SCROLL_EDGE_PX, 1) * AUTO_SCROLL_MAX_STEP_PX);
    }
  };

  /**
   * Follows one pointer until it is released. `capture` keeps it reporting
   * outside the window; only a started drag or rectangle takes it, because a
   * captured press no longer clicks the row it started on.
   */
  const track = (
    event: PointerEvent,
    capture: boolean,
    handlers: { move: (event: PointerEvent) => void; up: (event: PointerEvent) => void; cancel?: () => void }
  ) => {
    const root = options.root();
    if (!root) return;
    endGesture?.();
    const pointerId = event.pointerId;
    if (capture) {
      try {
        root.setPointerCapture(pointerId);
      } catch {
        // A pointer that is no longer down cannot be captured; the window listeners still follow it.
      }
    }
    const onMove = (next: PointerEvent) => next.pointerId === pointerId && handlers.move(next);
    const onUp = (next: PointerEvent) => {
      if (next.pointerId !== pointerId) return;
      finish();
      handlers.up(next);
    };
    const onCancel = (next: PointerEvent) => {
      if (next.pointerId !== pointerId) return;
      finish();
      handlers.cancel?.();
    };
    const finish = () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onCancel);
      endGesture = undefined;
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onCancel);
    endGesture = () => {
      finish();
      handlers.cancel?.();
    };
  };

  /** `event` is the press the drag grew out of, `current` the move that crossed the threshold. */
  const startDrag = (sourceIndex: number, event: PointerEvent, current: PointerEvent) => {
    const root = options.root();
    const sourceRow = root ? rowElements(root)[sourceIndex] : undefined;
    if (!root || !sourceRow) return;
    cancelRipple();
    suppressClick = true;
    const indices = options.beginMove(sourceIndex);

    // The ghost is a copy of the grabbed row that follows the pointer.
    const box = sourceRow.getBoundingClientRect();
    const offsetX = event.clientX - box.left;
    const offsetY = event.clientY - box.top;
    const ghost = sourceRow.cloneNode(true) as HTMLElement;
    ghost.removeAttribute('data-queue-index');
    Object.assign(ghost.style, {
      position: 'fixed',
      left: '0',
      top: '0',
      width: `${box.width}px`,
      opacity: '0.8',
      pointerEvents: 'none',
      zIndex: '100',
    });
    ghost.classList.add('bg-primary-plane', 'shadow-lg');
    document.body.appendChild(ghost);

    let clientX = current.clientX;
    let clientY = current.clientY;
    let insertBefore: number | null = null;
    let frame = 0;
    const update = () => {
      ghost.style.transform = `translate(${clientX - offsetX}px, ${clientY - offsetY}px)`;
      insertBefore = insertBeforeAt(clientY);
      if (insertBefore === null || isQueueMoveNoop(indices, insertBefore)) {
        setDropIndicator(null);
      } else {
        const length = options.queueLength();
        setDropIndicator(insertBefore < length ? { index: insertBefore, edge: 'top' } : { index: length - 1, edge: 'bottom' });
      }
    };
    const tick = () => {
      autoScroll(clientY);
      update();
      frame = requestAnimationFrame(tick);
    };
    const stop = () => {
      cancelAnimationFrame(frame);
      ghost.remove();
      setDropIndicator(null);
    };
    update();
    frame = requestAnimationFrame(tick);

    track(event, true, {
      move: (next) => {
        clientX = next.clientX;
        clientY = next.clientY;
        update();
      },
      up: (next) => {
        clientX = next.clientX;
        clientY = next.clientY;
        update();
        stop();
        if (insertBefore !== null && !isQueueMoveNoop(indices, insertBefore)) {
          options.move(indices, insertBefore);
        }
      },
      cancel: stop,
    });
  };

  const startRect = (startIndex: number | null, event: PointerEvent, current: PointerEvent) => {
    const root = options.root();
    if (!root) return;
    cancelRipple();
    suppressClick = true;
    const anchor = startIndex ?? options.queueLength() - 1;
    const rootBox = root.getBoundingClientRect();
    const start = { x: event.clientX - rootBox.left, y: event.clientY - rootBox.top };

    const update = (clientX: number, clientY: number) => {
      // Root-relative content coordinates; the point is kept inside the visible list.
      const box = root.getBoundingClientRect();
      const visible = options.scrollContainer()?.getBoundingClientRect() ?? box;
      const x = Math.min(Math.max(clientX, box.left), box.right) - box.left;
      const y = Math.min(Math.max(clientY, Math.max(box.top, visible.top)), Math.min(box.bottom, visible.bottom)) - box.top;
      const top = Math.min(start.y, y);
      const bottom = Math.max(start.y, y);
      setRect({ left: Math.min(start.x, x), top, width: Math.abs(x - start.x), height: bottom - top });

      const covered = rowElements(root)
        .map((row, index) => ({ index, top: row.offsetTop, bottom: row.offsetTop + row.offsetHeight }))
        .filter((row) => row.bottom > top && row.top <= bottom);
      options.selectRect(covered.length ? { from: covered[0].index, to: covered[covered.length - 1].index } : null, anchor);
    };
    update(current.clientX, current.clientY);

    track(event, true, {
      move: (next) => update(next.clientX, next.clientY),
      up: () => setRect(null),
      // Leaving the window ends the rectangle; what it selected stays selected.
      cancel: () => setRect(null),
    });
  };

  /**
   * PC press on a row or the blank area. A press that travels becomes a
   * rectangle (with Shift) or a drag; one that does not stays a click.
   */
  const pressPC = (event: PointerEvent, index: number | null) => {
    if (event.button !== 0 || (event.shiftKey && event.ctrlKey)) return;
    if (!event.shiftKey && index === null) return;
    const startX = event.clientX;
    const startY = event.clientY;
    track(event, false, {
      move: (next) => {
        if (Math.hypot(next.clientX - startX, next.clientY - startY) < DRAG_THRESHOLD_PX) return;
        if (event.shiftKey) {
          startRect(index, event, next);
        } else if (index !== null) {
          startDrag(index, event, next);
        }
      },
      up: () => {},
    });
  };

  /** SP press on a row's move handle: drag when it travels, long-press when it is held. */
  const pressSPHandle = (event: PointerEvent, index: number, canLongPress: boolean) => {
    const startX = event.clientX;
    const startY = event.clientY;
    const timers: ReturnType<typeof setTimeout>[] = [];
    let longPressed = false;
    const clearTimers = () => {
      timers.forEach(clearTimeout);
      setLongPressPreview(null);
    };
    if (canLongPress) {
      timers.push(setTimeout(() => setLongPressPreview(index), LONG_PRESS_CHECKBOX_MS));
      timers.push(
        setTimeout(() => {
          longPressed = true;
          suppressClick = true;
          setLongPressPreview(null);
          options.longPress(index);
        }, LONG_PRESS_SELECT_MS)
      );
    }
    track(event, false, {
      move: (next) => {
        if (longPressed || Math.hypot(next.clientX - startX, next.clientY - startY) < DRAG_THRESHOLD_PX) return;
        clearTimers();
        startDrag(index, event, next);
      },
      up: clearTimers,
      cancel: clearTimers,
    });
  };

  /** True once for the click that follows a drag, rectangle or long-press. */
  const consumeClick = () => {
    const suppressed = suppressClick;
    suppressClick = false;
    return suppressed;
  };

  onCleanup(() => endGesture?.());

  return { dropIndicator, rect, longPressPreview, pressPC, pressSPHandle, consumeClick, resetClick: () => (suppressClick = false) };
}
