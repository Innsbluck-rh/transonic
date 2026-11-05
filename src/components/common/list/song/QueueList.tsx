import { Component, createEffect, createMemo, createSignal, For, onCleanup, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import SongItem from '~/components/common/list/song/SongItem';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
import { resolveSongItemConditions } from '~/features/items/SongItemConditions';
import { buildQueueItemMenuItems, buildQueueSelectionMenuItems, showContextMenu } from '~/features/menu';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { hasPlaybackCommandError } from '~/features/playback/service';
import { usePlayback } from '~/features/playback/usePlayback';
import {
  addToQueueSelection,
  clearQueueSelection,
  isQueueIndexSelected,
  isQueueSelecting,
  queueSelection,
  selectQueueRangeTo,
  setQueueSelectionSpan,
  toggleQueueSelection,
} from '~/stores/QueueSelectionStore';
import { isSP } from '~/utils/isSP';
import { createQueueListGestures, queueRowIndexAt } from './queueListGestures';

interface QueueListProps {
  queue: SongResponse[];
  /** Scrolled while a dragged row nears its top or bottom edge. */
  scrollContainer?: HTMLElement;
  itemRef?: (index: number, element: HTMLDivElement | undefined) => void;
  onClickQueue?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
}

function queueCoverArtId(song: SongResponse) {
  return song.displayCoverArtId ?? song.coverArtId;
}

const QueueList: Component<QueueListProps> = (props) => {
  const navigate = useSPNavigate();
  const { isPlayNextQueueIndex, status: playbackStatus } = usePlayback();
  const requestedCoverArtIdSet = new Set<string>();
  const [requestedCoverArtIds, setRequestedCoverArtIds] = createSignal<readonly string[]>([]);
  let rootElement: HTMLDivElement | undefined;
  let coverArtObserver: IntersectionObserver | undefined;

  const gestures = createQueueListGestures({
    root: () => rootElement,
    scrollContainer: () => props.scrollContainer,
    queueLength: () => props.queue.length,
    beginMove: (index) => {
      if (!isQueueSelecting()) {
        return [index];
      }
      if (!isQueueIndexSelected(index)) {
        addToQueueSelection(index);
      }
      return queueSelection().indices;
    },
    move: (indices, insertBefore) => {
      commands.playbackMoveQueueIndices({ indices, insertBefore }).then(hasPlaybackCommandError);
    },
    selectRect: setQueueSelectionSpan,
    longPress: (index) => {
      if (!isQueueSelecting()) {
        toggleQueueSelection(index);
      }
    },
  });

  const requestCoverArtId = (coverArtId: string | null | undefined) => {
    const normalizedId = coverArtId?.trim();
    if (!normalizedId || requestedCoverArtIdSet.has(normalizedId)) {
      return;
    }

    requestedCoverArtIdSet.add(normalizedId);
    setRequestedCoverArtIds([...requestedCoverArtIdSet]);
  };

  const ensureCoverArtObserver = () => {
    if (typeof IntersectionObserver === 'undefined') {
      return undefined;
    }
    if (!coverArtObserver) {
      coverArtObserver = new IntersectionObserver((entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) {
            continue;
          }

          const coverArtId = (entry.target as HTMLElement).dataset.coverArtId;
          requestCoverArtId(coverArtId);
          coverArtObserver?.unobserve(entry.target);
        }
      });
    }

    return coverArtObserver;
  };

  createEffect<string | undefined>((previousQueueKey) => {
    const nextQueueKey = props.queue.map((song) => song.id).join('\0');
    if (previousQueueKey !== undefined && previousQueueKey !== nextQueueKey) {
      requestedCoverArtIdSet.clear();
      setRequestedCoverArtIds([]);
    }

    return nextQueueKey;
  }, undefined);

  // PC: Esc ends the selection.
  createEffect(() => {
    if (isSP() || !isQueueSelecting()) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        clearQueueSelection();
      }
    };
    document.addEventListener('keydown', onKeyDown);
    onCleanup(() => document.removeEventListener('keydown', onKeyDown));
  });

  onCleanup(() => {
    coverArtObserver?.disconnect();
  });

  const coverArtMap = useCoverArtMap(requestedCoverArtIds, CoverArtSizes.sm, {
    cachedFallbackSizes: [CoverArtSizes.lg, CoverArtSizes.md],
  });

  const onPointerDown = (event: PointerEvent) => {
    gestures.resetClick();
    const index = queueRowIndexAt(event.target);
    if (!isSP()) {
      gestures.pressPC(event, index);
    } else if (index !== null) {
      gestures.pressSP(event, index);
    }
  };

  // While rows are selected a click on a row only turns it on or off; otherwise it plays the row.
  const onClick = (event: MouseEvent) => {
    if (gestures.consumeClick()) {
      return;
    }
    const index = queueRowIndexAt(event.target);
    if (index === null) {
      // PC: a plain click on the list below the rows ends the selection.
      if (!isSP() && !event.ctrlKey && !event.shiftKey) {
        clearQueueSelection();
      }
      return;
    }

    if (!isSP() && event.shiftKey) {
      if (!event.ctrlKey) {
        selectQueueRangeTo(index);
      }
      return;
    }
    if (isQueueSelecting() || (!isSP() && event.ctrlKey)) {
      toggleQueueSelection(index);
      return;
    }

    const entry = props.queue[index];
    if (!entry) return;
    props.onClickQueue?.(entry, index, props.queue);
    commands.playbackPlayQueueIndex({ index });
  };

  const onContextMenuCapture = (event: MouseEvent) => {
    // SP: a long-press lifts the row, so the system's long-press menu must not open; row menus come from the ⋮ button.
    if (isSP()) {
      event.preventDefault();
      return;
    }
    // PC: while rows are selected, a right-click anywhere in the list opens the selection menu instead of the row's own.
    if (isQueueSelecting()) {
      showContextMenu(event, buildQueueSelectionMenuItems(queueSelection().indices));
    }
  };

  return (
    <div
      ref={(element) => {
        rootElement = element;
        element.addEventListener('contextmenu', onContextMenuCapture, true);
      }}
      class='relative flex w-full flex-1 flex-col select-none'
      onPointerDown={onPointerDown}
      onClick={onClick}
      // The browser's own image drag would take the pointer away from a row drag.
      onDragStart={(event) => event.preventDefault()}
    >
      <For
        each={props.queue}
        fallback={
          <div class='flex w-full p-2'>
            <p class='text-secondary-text px-1 text-xs italic'>[no songs]</p>
          </div>
        }
      >
        {(entry, i) => {
          const coverArtId = createMemo(() => queueCoverArtId(entry));
          const dropIndicatorEdge = () => {
            const current = gestures.dropIndicator();
            return current?.index === i() ? current.edge : null;
          };
          let itemElement: HTMLDivElement | undefined;
          onCleanup(() => {
            if (itemElement) {
              coverArtObserver?.unobserve(itemElement);
            }
          });

          const setItemElement = (element: HTMLDivElement) => {
            itemElement = element;
            props.itemRef?.(i(), element);

            const normalizedId = coverArtId()?.trim();
            if (!normalizedId) {
              return;
            }

            const observer = ensureCoverArtObserver();
            if (!observer) {
              requestCoverArtId(normalizedId);
              return;
            }

            element.dataset.coverArtId = normalizedId;
            observer.observe(element);
          };

          const conditions = () =>
            resolveSongItemConditions({
              playbackStatus: playbackStatus(),
              songId: entry.id,
              queueIndex: i(),
            });

          const menuItems = buildQueueItemMenuItems(entry, i, navigate);

          return (
            <div class='border-secondary-border border-b' data-queue-index={i()}>
              <SongItem
                ref={setItemElement}
                song={entry}
                playing={conditions().current}
                index={i() + 1}
                checked={isQueueSelecting() ? isQueueIndexSelected(i()) : undefined}
                coverArt
                coverArtUrl={coverArtMap.src(coverArtId())}
                emphasizeTitle
                playNext={isPlayNextQueueIndex(i())}
                conditions={conditions()}
                // SP opens the row menu from the ⋮ button; a long-press on the row lifts it instead.
                menuItems={isSP() ? undefined : menuItems}
                withMenuButton={isSP()}
                onMenuClick={(event) => showContextMenu(event, menuItems)}
              >
                <Show when={dropIndicatorEdge()}>
                  {(edge) => (
                    <div
                      class='bg-accent pointer-events-none absolute right-0 left-0 z-20 h-0.5'
                      classList={{
                        'top-0': edge() === 'top',
                        'bottom-0': edge() === 'bottom',
                      }}
                    />
                  )}
                </Show>
              </SongItem>
            </div>
          );
        }}
      </For>
      <Show when={gestures.rect()}>
        {(rect) => (
          <div
            class='border-accent bg-accent/15 pointer-events-none absolute z-30 border'
            style={{ left: `${rect().left}px`, top: `${rect().top}px`, width: `${rect().width}px`, height: `${rect().height}px` }}
          />
        )}
      </Show>
    </div>
  );
};

export default QueueList;
