import { createSignal, onCleanup } from 'solid-js';
import { isQueueMoveNoop } from '~/features/playback/queueSelection';
import { cancelRipple } from '~/utils/ripple';

/** Distance a pointer has to travel before a press becomes a drag or a rectangle. */
const DRAG_THRESHOLD_PX = 5;
/** SP: holding a row this long lifts it; moving then drags it, letting go selects it. */
export const LONG_PRESS_MS = 450;
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
  /** SP: a long-press on a row that does not lift, or a lifted row let go without moving. */
  longPress: (index: number) => void;
}

function rowElements(root: HTMLElement) {
  return [...root.querySelectorAll<HTMLElement>('[data-queue-index]')];
}

export function queueRowIndexAt(target: EventTarget | null): number | null {
  const row = target instanceof Element ? target.closest<HTMLElement>('[data-queue-index]') : null;
  return row ? Number(row.dataset.queueIndex) : null;
}

/**
 * Pointer gestures of the queue list that a click cannot express: dragging rows
 * (PC: press and move, SP: long-press and move), the PC Shift+drag rectangle
 * and the SP long-press that selects. Clicks and taps stay with the list's
 * click handler; `consumeClick` tells it when a gesture used them up.
 */
export function createQueueListGestures(options: QueueListGestureOptions) {
  const [dropIndicator, setDropIndicator] = createSignal<QueueDropIndicator | null>(null);
  const [rect, setRect] = createSignal<QueueRect | null>(null);
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

  /**
   * Shows the grabbed row under the pointer and moves rows where it is let go.
   * `event` is the press, `current` where the pointer is now. A `lifted` row
   * (SP long-press) only becomes a drag once it moves; let go before that, it
   * counts as a long-press.
   */
  const startDrag = (sourceIndex: number, event: PointerEvent, current: PointerEvent, lifted: boolean) => {
    const root = options.root();
    const sourceRow = root ? rowElements(root)[sourceIndex] : undefined;
    if (!root || !sourceRow) return;
    cancelRipple();
    suppressClick = true;
    let indices = lifted ? null : options.beginMove(sourceIndex);

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

    // Once a row is lifted the finger drags it instead of scrolling the list.
    const preventScroll = (touch: TouchEvent) => touch.cancelable && touch.preventDefault();
    window.addEventListener('touchmove', preventScroll, { passive: false });

    const liftX = current.clientX;
    const liftY = current.clientY;
    let clientX = current.clientX;
    let clientY = current.clientY;
    let insertBefore: number | null = null;
    let frame = 0;
    const update = () => {
      ghost.style.transform = `translate(${clientX - offsetX}px, ${clientY - offsetY}px)`;
      if (!indices && Math.hypot(clientX - liftX, clientY - liftY) >= DRAG_THRESHOLD_PX) {
        indices = options.beginMove(sourceIndex);
      }
      insertBefore = indices ? insertBeforeAt(clientY) : null;
      if (!indices || insertBefore === null || isQueueMoveNoop(indices, insertBefore)) {
        setDropIndicator(null);
      } else {
        const length = options.queueLength();
        setDropIndicator(insertBefore < length ? { index: insertBefore, edge: 'top' } : { index: length - 1, edge: 'bottom' });
      }
    };
    const tick = () => {
      if (indices) {
        autoScroll(clientY);
      }
      update();
      frame = requestAnimationFrame(tick);
    };
    const stop = () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('touchmove', preventScroll);
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
        if (!indices) {
          options.longPress(sourceIndex);
        } else if (insertBefore !== null && !isQueueMoveNoop(indices, insertBefore)) {
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
    // The rectangle stays inside the visible list, but the list scrolls under it
    // while the pointer is near or past its top or bottom edge, as for a drag.
    let clientX = current.clientX;
    let clientY = current.clientY;
    let frame = 0;
    const tick = () => {
      autoScroll(clientY);
      update(clientX, clientY);
      frame = requestAnimationFrame(tick);
    };
    const stop = () => {
      cancelAnimationFrame(frame);
      setRect(null);
    };
    update(clientX, clientY);
    frame = requestAnimationFrame(tick);

    track(event, true, {
      move: (next) => {
        clientX = next.clientX;
        clientY = next.clientY;
        update(clientX, clientY);
      },
      up: stop,
      // Leaving the window ends the rectangle; what it selected stays selected.
      cancel: stop,
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
          startDrag(index, event, next, false);
        }
      },
      up: () => {},
    });
  };

  /**
   * SP press on a row. Moving before the long-press time is a scroll and lets
   * the press go. Held still, a `liftable` row lifts (see `startDrag`); any
   * other row counts as a long-press right away.
   */
  const pressSP = (event: PointerEvent, index: number, liftable: boolean) => {
    let latest = event;
    const timer = setTimeout(() => {
      if (liftable) {
        startDrag(index, event, latest, true);
        return;
      }
      endGesture?.();
      cancelRipple();
      suppressClick = true;
      options.longPress(index);
    }, LONG_PRESS_MS);
    track(event, false, {
      move: (next) => {
        latest = next;
        if (Math.hypot(next.clientX - event.clientX, next.clientY - event.clientY) >= DRAG_THRESHOLD_PX) {
          endGesture?.();
        }
      },
      up: () => clearTimeout(timer),
      cancel: () => clearTimeout(timer),
    });
  };

  /** True once for the click that follows a drag, rectangle or long-press. */
  const consumeClick = () => {
    const suppressed = suppressClick;
    suppressClick = false;
    return suppressed;
  };

  onCleanup(() => endGesture?.());

  return { dropIndicator, rect, pressPC, pressSP, consumeClick, resetClick: () => (suppressClick = false) };
}
