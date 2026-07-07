import { combine } from '@atlaskit/pragmatic-drag-and-drop/combine';
import { draggable, dropTargetForElements, type ElementDragPayload } from '@atlaskit/pragmatic-drag-and-drop/element/adapter';
import { Icon } from '@iconify-icon/solid';
import { Component, createEffect, createMemo, createSignal, For, onCleanup, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import SongItem from '~/components/common/list/song/SongItem';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
import { resolveSongItemConditions } from '~/features/items/SongItemConditions';
import { buildQueueItemMenuItems } from '~/features/menu';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { hasPlaybackCommandError } from '~/features/playback/service';
import { usePlayback } from '~/features/playback/usePlayback';

interface QueueListProps {
  queue: SongResponse[];
  itemRef?: (index: number, element: HTMLDivElement | undefined) => void;
  onClickQueue?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
}

const QUEUE_COVER_ART_ROOT_MARGIN = '480px 0px';
const QUEUE_ITEM_DND_TYPE = 'queue-item';
const QUEUE_ITEM_DROP_TARGET_TYPE = 'queue-item-drop-target';

type QueueDropEdge = 'top' | 'bottom';

interface QueueDropIndicator {
  index: number;
  edge: QueueDropEdge;
}

type QueueItemDragData = Record<string, unknown> & {
  type: typeof QUEUE_ITEM_DND_TYPE;
  index: number;
};

type QueueItemDropTargetData = Record<string | symbol, unknown> & {
  type: typeof QUEUE_ITEM_DROP_TARGET_TYPE;
  index: number;
  edge: QueueDropEdge;
};

function queueCoverArtId(song: SongResponse) {
  return song.displayCoverArtId ?? song.coverArtId;
}

function isQueueIndex(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0;
}

function isQueueItemDragData(data: ElementDragPayload['data']): data is QueueItemDragData {
  return data.type === QUEUE_ITEM_DND_TYPE && isQueueIndex(data.index);
}

function isQueueItemDropTargetData(data: Record<string | symbol, unknown>): data is QueueItemDropTargetData {
  return data.type === QUEUE_ITEM_DROP_TARGET_TYPE && isQueueIndex(data.index) && (data.edge === 'top' || data.edge === 'bottom');
}

function closestVerticalEdge(clientY: number, element: Element): QueueDropEdge {
  const rect = element.getBoundingClientRect();
  return clientY < rect.top + rect.height / 2 ? 'top' : 'bottom';
}

function moveDestinationIndex(sourceIndex: number, targetIndex: number, edge: QueueDropEdge, queueLength: number) {
  const insertionIndex = Math.min(Math.max(edge === 'top' ? targetIndex : targetIndex + 1, 0), queueLength);
  const destinationIndex = sourceIndex < insertionIndex ? insertionIndex - 1 : insertionIndex;
  return Math.min(Math.max(destinationIndex, 0), queueLength - 1);
}

const QueueList: Component<QueueListProps> = (props) => {
  const navigate = useSPNavigate();
  const { isPlayNextQueueIndex, status: playbackStatus } = usePlayback();
  const requestedCoverArtIdSet = new Set<string>();
  const [requestedCoverArtIds, setRequestedCoverArtIds] = createSignal<readonly string[]>([]);
  const [dropIndicator, setDropIndicator] = createSignal<QueueDropIndicator | null>(null);
  let coverArtObserver: IntersectionObserver | undefined;
  let suppressNextClick = false;
  let suppressClickResetId: number | undefined;

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
      coverArtObserver = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (!entry.isIntersecting) {
              continue;
            }

            const coverArtId = (entry.target as HTMLElement).dataset.coverArtId;
            requestCoverArtId(coverArtId);
            coverArtObserver?.unobserve(entry.target);
          }
        },
        { rootMargin: QUEUE_COVER_ART_ROOT_MARGIN }
      );
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

  const suppressFollowingClick = () => {
    suppressNextClick = true;
    if (suppressClickResetId !== undefined) {
      window.clearTimeout(suppressClickResetId);
      suppressClickResetId = undefined;
    }
  };

  const resetSuppressFollowingClickSoon = () => {
    if (suppressClickResetId !== undefined) {
      window.clearTimeout(suppressClickResetId);
    }
    suppressClickResetId = window.setTimeout(() => {
      suppressNextClick = false;
      suppressClickResetId = undefined;
    }, 250);
  };

  const consumeSuppressedClick = () => {
    if (!suppressNextClick) {
      return false;
    }
    suppressNextClick = false;
    if (suppressClickResetId !== undefined) {
      window.clearTimeout(suppressClickResetId);
      suppressClickResetId = undefined;
    }
    return true;
  };

  onCleanup(() => {
    coverArtObserver?.disconnect();
    if (suppressClickResetId !== undefined) {
      window.clearTimeout(suppressClickResetId);
    }
  });

  const coverArtMap = useCoverArtMap(requestedCoverArtIds, CoverArtSizes.sm, {
    cachedFallbackSizes: [CoverArtSizes.lg, CoverArtSizes.md],
  });

  return (
    <div class='flex w-full flex-col'>
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
            const current = dropIndicator();
            return current?.index === i() ? current.edge : null;
          };
          let itemElement: HTMLDivElement | undefined;
          let itemDndCleanup: (() => void) | undefined;
          onCleanup(() => {
            itemDndCleanup?.();
            if (itemElement) {
              coverArtObserver?.unobserve(itemElement);
            }
          });

          const updateDropIndicator = (source: ElementDragPayload, targetData: Record<string | symbol, unknown>) => {
            const sourceIndex = isQueueItemDragData(source.data) ? source.data.index : null;
            if (sourceIndex === null || !isQueueItemDropTargetData(targetData)) {
              setDropIndicator(null);
              return;
            }

            const destinationIndex = moveDestinationIndex(sourceIndex, targetData.index, targetData.edge, props.queue.length);
            setDropIndicator(destinationIndex === sourceIndex ? null : { index: targetData.index, edge: targetData.edge });
          };

          const setItemElement = (element: HTMLDivElement) => {
            itemElement = element;
            props.itemRef?.(i(), element);
            itemDndCleanup?.();
            itemDndCleanup = combine(
              draggable({
                element,
                getInitialData: () => ({ type: QUEUE_ITEM_DND_TYPE, index: i() }),
                onDragStart: () => {
                  suppressFollowingClick();
                },
                onDrop: () => {
                  setDropIndicator(null);
                  resetSuppressFollowingClickSoon();
                },
              }),
              dropTargetForElements({
                element,
                canDrop: ({ source }) => props.queue.length > 1 && isQueueItemDragData(source.data),
                getData: ({ input, element }) => ({
                  type: QUEUE_ITEM_DROP_TARGET_TYPE,
                  index: i(),
                  edge: closestVerticalEdge(input.clientY, element),
                }),
                onDrag: ({ source, self }) => updateDropIndicator(source, self.data),
                onDragEnter: ({ source, self }) => updateDropIndicator(source, self.data),
                onDragLeave: ({ self }) => {
                  if (isQueueItemDropTargetData(self.data) && dropIndicator()?.index === self.data.index) {
                    setDropIndicator(null);
                  }
                },
                onDrop: ({ source, self }) => {
                  setDropIndicator(null);
                  const fromIndex = isQueueItemDragData(source.data) ? source.data.index : null;
                  if (fromIndex === null || !isQueueItemDropTargetData(self.data)) {
                    return;
                  }

                  const toIndex = moveDestinationIndex(fromIndex, self.data.index, self.data.edge, props.queue.length);
                  if (toIndex === fromIndex) {
                    return;
                  }

                  commands
                    .playbackMoveQueueIndex({
                      fromIndex,
                      toIndex,
                    })
                    .then(hasPlaybackCommandError);
                },
              })
            );

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

          const onClickEntry = () => {
            if (consumeSuppressedClick()) {
              return;
            }
            if (!props.queue) return;
            props.onClickQueue?.(entry, i(), props.queue);

            commands.playbackPlayQueueIndex({
              index: i(),
            });
          };
          const conditions = () =>
            resolveSongItemConditions({
              playbackStatus: playbackStatus(),
              songId: entry.id,
              queueIndex: i(),
            });

          return (
            <div class='border-secondary-border border-b'>
              <SongItem
                ref={setItemElement}
                song={entry}
                leadingContent={
                  conditions().current ? (
                    <Icon class='song-item-leading -ml-0.5 w-fit scale-125 text-xs' icon='material-symbols:play-arrow' />
                  ) : (
                    String(i() + 1)
                  )
                }
                coverArt
                coverArtUrl={coverArtMap.src(coverArtId())}
                emphasizeTitle
                playNext={isPlayNextQueueIndex(i())}
                conditions={conditions()}
                menuItems={buildQueueItemMenuItems(entry, i, navigate)}
                onClick={onClickEntry}
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
    </div>
  );
};

export default QueueList;
