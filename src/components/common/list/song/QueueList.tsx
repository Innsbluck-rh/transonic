import { Component, createEffect, createMemo, createSignal, For, on, onCleanup, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import QueueMoveGroupItem from '~/components/common/list/song/QueueMoveGroupItem';
import SongItem from '~/components/common/list/song/SongItem';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
import { buildQueueItemMenuItems, buildQueueSelectionMenuItems, showContextMenu } from '~/features/menu';
import type { MenuItem } from '~/features/menu/types';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { isCurrentSong } from '~/features/playback/currentSong';
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
    selectedIndices: () => queueSelection().indices,
    selectRect: setQueueSelectionSpan,
  });
  // A preview kept after letting go of a drag lasts until the moved queue arrives.
  createEffect(on(() => props.queue, gestures.clearDragPreview, { defer: true }));
  const movingIndexSet = createMemo(() => new Set(gestures.dragPreview()?.indices));

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

  /** A row's song as the list draws it; the drag preview draws a single moved row with it too. */
  const renderSongItem = (entry: SongResponse, index: () => number, menuItems?: MenuItem[]) => (
    <SongItem
      song={entry}
      playing={isCurrentSong({ playbackStatus: playbackStatus(), songId: entry.id, queueIndex: index() })}
      index={index() + 1}
      indexContentMode={isQueueSelecting() ? 'selection' : 'indexOrPlaying'}
      selected={isQueueIndexSelected(index())}
      coverArt
      coverArtUrl={coverArtMap.src(queueCoverArtId(entry))}
      boldTitle='always'
      playNext={isPlayNextQueueIndex(index())}
      // SP opens the row menu from the ⋮ button; a long-press on the row lifts it instead.
      contextMenuItems={isSP() ? undefined : menuItems}
      withMenuButton={isSP()}
      onMenuButtonClick={(event) => menuItems && showContextMenu(event, menuItems)}
    />
  );

  /**
   * While rows are dragged and would land before `insertBefore`, draws them inserted
   * there: one row as itself, several as one group row. It is not a
   * `data-queue-index` row, so gestures and clicks see only the real rows.
   */
  const MovePreview = (previewProps: { insertBefore: number }) => {
    const moving = () => {
      const preview = gestures.dragPreview();
      return preview?.insertBefore === previewProps.insertBefore ? preview.indices : [];
    };
    const songs = () => moving().flatMap((index) => props.queue[index] ?? []);
    return (
      <Show when={songs().length > 0}>
        <div class='border-secondary-border bg-primary-plane pointer-events-none relative border-b' data-queue-move-preview>
          <Show when={songs().length === 1} fallback={<QueueMoveGroupItem count={songs().length} />}>
            {renderSongItem(songs()[0], () => moving()[0])}
          </Show>
        </div>
      </Show>
    );
  };

  const onPointerDown = (event: PointerEvent) => {
    gestures.resetClick();
    const index = queueRowIndexAt(event.target);
    if (!isSP()) {
      gestures.pressPC(event, index);
    } else if (index !== null) {
      gestures.pressSP(event, index, isQueueIndexSelected(index));
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
      // isolate: the list's own layers (selection bar, rectangle) stay under whatever overlays the list, such as the selection toolbar.
      class='relative isolate flex w-full flex-1 flex-col select-none'
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

          const menuItems = buildQueueItemMenuItems(entry, i, navigate);

          return (
            <>
              <MovePreview insertBefore={i()} />
              <div
                ref={setItemElement}
                class='border-secondary-border relative border-b'
                // A row being dragged stays in place, faded; the preview draws it where it would land.
                classList={{ 'opacity-25': movingIndexSet().has(i()) }}
                data-queue-index={i()}
              >
                {renderSongItem(entry, i, menuItems)}
              </div>
            </>
          );
        }}
      </For>
      <MovePreview insertBefore={props.queue.length} />
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
