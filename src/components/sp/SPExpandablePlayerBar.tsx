import { createEffect, createMemo, createSignal, JSX, on, onCleanup, onMount, ParentComponent, Show } from 'solid-js';
import PlayerBar from '~/components/player/PlayerBar';
import SPPlayer from './SPPlayer';

// ── 外部制御用のリアクティブブリッジ ──────────────────────────────

interface PlayerBarRequest {
  expanded: boolean;
  immediate: boolean;
}

const [playerBarRequest, setPlayerBarRequest] = createSignal<PlayerBarRequest | null>(null);

export function openPlayerBar() {
  setPlayerBarRequest({ expanded: true, immediate: false });
}

export function closePlayerBar(immediate = false) {
  setPlayerBarRequest({ expanded: false, immediate });
}

// ── コンポーネント本体 ──────────────────────────────────────────

interface SPExpandablePlayerBarProps {
  topOffsetPx?: number;
  bottomOffsetPx?: number;
}

type SheetPhase = 'collapsed' | 'dragging' | 'expanded';
type DragDirection = 'expand' | 'collapse';

const DEFAULT_SETTLE_DURATION_MS = 220;
const OPEN_VELOCITY_THRESHOLD = -600;
const CLOSE_VELOCITY_THRESHOLD = 600;
const DRAG_CLICK_SUPPRESSION_THRESHOLD_PX = 2;

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

const SPExpandablePlayerBar: ParentComponent<SPExpandablePlayerBarProps> = (props) => {
  let containerRef: HTMLDivElement | undefined;
  let collapsedBarHostRef: HTMLDivElement | undefined;

  const [phase, setPhase] = createSignal<SheetPhase>('collapsed');
  const [containerHeight, setContainerHeight] = createSignal(0);
  const [translateY, setTranslateY] = createSignal(0);
  const [skipAnimation, setSkipAnimation] = createSignal(false);
  const [dragPointerId, setDragPointerId] = createSignal<number | null>(null);
  const [dragStartClientY, setDragStartClientY] = createSignal(0);
  const [dragStartTranslateY, setDragStartTranslateY] = createSignal(0);
  const [lastClientY, setLastClientY] = createSignal(0);
  const [lastTimestamp, setLastTimestamp] = createSignal(0);
  const [velocityY, setVelocityY] = createSignal(0);
  const [suppressBarClick, setSuppressBarClick] = createSignal(false);

  const topOffsetPx = createMemo(() => Math.max(0, props.topOffsetPx ?? 0));
  const bottomOffsetPx = createMemo(() => Math.max(0, props.bottomOffsetPx ?? 0));

  const closedTranslateY = createMemo(() => Math.max(containerHeight() - topOffsetPx(), 0));
  const progress = createMemo(() => {
    const nextClosedTranslateY = closedTranslateY();
    if (nextClosedTranslateY <= 0) {
      return 1;
    }

    return 1 - translateY() / nextClosedTranslateY;
  });

  const isDragging = createMemo(() => phase() === 'dragging');
  const shouldAnimate = createMemo(() => !isDragging() && !skipAnimation());
  const shouldRenderFullPlayer = createMemo(() => phase() !== 'collapsed' || translateY() < closedTranslateY());

  const openSheet = () => {
    setPhase('expanded');
    setTranslateY(0);
  };

  const closeSheet = () => {
    setPhase('collapsed');
    setTranslateY(closedTranslateY());
  };

  const closeSheetInstantly = () => {
    setSkipAnimation(true);
    setPhase('collapsed');
    setTranslateY(closedTranslateY());
    requestAnimationFrame(() => setSkipAnimation(false));
  };

  const syncContainerHeight = () => {
    const nextHeight = containerRef?.getBoundingClientRect().height ?? 0;
    setContainerHeight(Math.round(nextHeight));
  };

  const beginDrag =
    (direction: DragDirection): JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> =>
    (event) => {
      if (dragPointerId() !== null) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();

      const startTranslateY = direction === 'expand' ? closedTranslateY() : translateY();
      const now = performance.now();

      setPhase('dragging');
      setDragPointerId(event.pointerId);
      setDragStartClientY(event.clientY);
      setDragStartTranslateY(startTranslateY);
      setLastClientY(event.clientY);
      setLastTimestamp(now);
      setVelocityY(0);
      setSuppressBarClick(false);
      setTranslateY(startTranslateY);

      event.currentTarget.setPointerCapture(event.pointerId);
    };

  const handleDragMove: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (dragPointerId() !== event.pointerId || !isDragging()) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const deltaY = event.clientY - dragStartClientY();
    const nextTranslateY = clamp(dragStartTranslateY() + deltaY, 0, closedTranslateY());
    const now = performance.now();
    const elapsedMs = now - lastTimestamp();

    if (Math.abs(deltaY) >= DRAG_CLICK_SUPPRESSION_THRESHOLD_PX) {
      setSuppressBarClick(true);
    }

    if (elapsedMs > 0) {
      setVelocityY(((event.clientY - lastClientY()) / elapsedMs) * 1000);
    }

    setLastClientY(event.clientY);
    setLastTimestamp(now);
    setTranslateY(nextTranslateY);
  };

  const finishDrag = (pointerId: number | null, currentTarget: HTMLDivElement, cancelled: boolean) => {
    if (pointerId === null || dragPointerId() !== pointerId) {
      return;
    }

    if (currentTarget.hasPointerCapture(pointerId)) {
      currentTarget.releasePointerCapture(pointerId);
    }

    const nextVelocityY = cancelled ? 0 : velocityY();
    const nextTranslateY = translateY();
    const nextClosedTranslateY = closedTranslateY();
    const nextProgress = nextClosedTranslateY <= 0 ? 1 : 1 - nextTranslateY / nextClosedTranslateY;

    setDragPointerId(null);

    if (cancelled) {
      if (nextProgress >= 0.5) {
        openSheet();
      } else {
        closeSheet();
      }

      return;
    }

    if (nextVelocityY <= OPEN_VELOCITY_THRESHOLD) {
      openSheet();
      return;
    }

    if (nextVelocityY >= CLOSE_VELOCITY_THRESHOLD) {
      closeSheet();
      return;
    }

    if (nextProgress >= 0.5) {
      openSheet();
      return;
    }

    closeSheet();
  };

  const handleDragEnd: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    finishDrag(event.pointerId, event.currentTarget, false);
  };

  const handleDragCancel: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    finishDrag(event.pointerId, event.currentTarget, true);
  };

  const handleCollapsedBarClickCapture = (event: Event) => {
    if (!suppressBarClick()) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    setSuppressBarClick(false);
  };

  createEffect(() => {
    const nextClosedTranslateY = closedTranslateY();

    if (phase() === 'collapsed') {
      setTranslateY(nextClosedTranslateY);
      return;
    }

    if (phase() === 'expanded') {
      setTranslateY(0);
    }
  });

  // 外部からのリクエストを監視してローカルstateに反映する
  createEffect(
    on(playerBarRequest, (request) => {
      if (!request) return;

      if (request.expanded && phase() !== 'expanded') {
        openSheet();
      } else if (!request.expanded && phase() !== 'collapsed') {
        if (request.immediate) {
          closeSheetInstantly();
        } else {
          closeSheet();
        }
      }
    })
  );

  onMount(() => {
    syncContainerHeight();

    const observer = new ResizeObserver(() => {
      syncContainerHeight();
    });

    if (containerRef) {
      observer.observe(containerRef);
    }

    if (collapsedBarHostRef) {
      collapsedBarHostRef.addEventListener('click', handleCollapsedBarClickCapture, true);
    }

    window.addEventListener('resize', syncContainerHeight);

    onCleanup(() => {
      observer.disconnect();
      collapsedBarHostRef?.removeEventListener('click', handleCollapsedBarClickCapture, true);
      window.removeEventListener('resize', syncContainerHeight);
    });
  });

  return (
    <div ref={containerRef} class='relative flex min-h-0 w-full flex-1 flex-col overflow-hidden'>
      <div class='flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden' style='view-transition-name: sp-screen'>
        {props.children}
      </div>

      <div
        class='relative z-30 shrink-0'
        style={{
          opacity: `${clamp(1 - progress() * 1.1, 0, 1)}`,
          transform: `translateY(${progress() * 12}px)`,
          transition: shouldAnimate()
            ? `opacity ${DEFAULT_SETTLE_DURATION_MS}ms ease-out, transform ${DEFAULT_SETTLE_DURATION_MS}ms ease-out`
            : 'none',
        }}
      >
        <div ref={collapsedBarHostRef} class='pointer-events-auto'>
          <PlayerBar
            iconsVisibility={{
              prev: false,
              playpause: true,
              next: false,
            }}
            onClickRestArea={openSheet}
            restAreaProps={{
              class: 'touch-none',
              onPointerDown: beginDrag('expand'),
              onPointerMove: handleDragMove,
              onPointerUp: handleDragEnd,
              onPointerCancel: handleDragCancel,
            }}
          />
        </div>
      </div>

      <Show when={shouldRenderFullPlayer()}>
        <div
          class='pointer-events-none absolute inset-x-0 z-40 overflow-hidden'
          style={{
            top: `${topOffsetPx()}px`,
            bottom: `${bottomOffsetPx()}px`,
            transform: `translateY(${translateY()}px)`,
            transition: shouldAnimate() ? `transform ${DEFAULT_SETTLE_DURATION_MS}ms ease-out` : 'none',
          }}
        >
          <div class='bg-primary-plane absolute inset-0 overflow-hidden shadow-[0_-12px_48px_rgba(0,0,0,0.35)]'>
            <div class='pointer-events-auto absolute inset-0 flex min-h-0 flex-col'>
              <SPPlayer
                topSectionProps={{
                  class: 'touch-none',
                  onPointerDown: beginDrag('collapse'),
                  onPointerMove: handleDragMove,
                  onPointerUp: handleDragEnd,
                  onPointerCancel: handleDragCancel,
                }}
              />
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default SPExpandablePlayerBar;
