import { Icon } from '@iconify-icon/solid';
import { Component, createEffect, createMemo, createSignal, For, onCleanup, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
import { resolveSongItemConditions, songItemConditionAttrs } from '~/features/items/SongItemConditions';
import { buildQueueItemMenuItems } from '~/features/menu';
import useContextMenu from '~/features/menu/useContextMenu';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { playbackStore } from '~/stores/PlaybackStore';
import { formatCompactDuration } from '~/utils/duration';

interface QueueListProps {
  queue: SongResponse[];
  itemRef?: (index: number, element: HTMLDivElement | undefined) => void;
  onClickQueue?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
}

const QUEUE_COVER_ART_ROOT_MARGIN = '480px 0px';

function queueCoverArtId(song: SongResponse) {
  return song.displayCoverArtId ?? song.coverArtId;
}

const QueueList: Component<QueueListProps> = (props) => {
  const navigate = useSPNavigate();
  const requestedCoverArtIdSet = new Set<string>();
  const [requestedCoverArtIds, setRequestedCoverArtIds] = createSignal<readonly string[]>([]);
  let coverArtObserver: IntersectionObserver | undefined;

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

  onCleanup(() => coverArtObserver?.disconnect());

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

          const onClickEntry = () => {
            if (!props.queue) return;
            props.onClickQueue?.(entry, i(), props.queue);

            commands.playbackPlayQueueIndex({
              index: i(),
            });
          };
          const contextMenuProps = useContextMenu(buildQueueItemMenuItems(entry, i(), navigate));
          const conditions = () =>
            resolveSongItemConditions({
              playbackStatus: playbackStore.status,
              songId: entry.id,
              queueIndex: i(),
            });

          return (
            <div
              ref={setItemElement}
              class='song-item ripple border-secondary-border flex h-auto cursor-pointer flex-row items-center overflow-x-hidden border-b px-4 py-2'
              onClick={onClickEntry}
              {...songItemConditionAttrs(conditions())}
              {...contextMenuProps}
            >
              <div class='flex w-7 flex-row items-start'>
                <Show
                  when={!conditions().current}
                  fallback={<Icon class='song-item-leading -ml-0.5 w-fit scale-125 text-xs' icon='material-symbols:play-arrow' />}
                >
                  <p class='song-item-leading archivo text-xs font-bold'>{i() + 1}</p>
                </Show>
              </div>

              <Show
                when={coverArtMap.src(coverArtId())}
                fallback={<div class='border-secondary-border mr-3 aspect-square max-h-8 w-8 rounded-md border' />}
              >
                {(assetUrl) => <img src={assetUrl()} class='mr-3 aspect-square max-h-8 w-8 rounded-md' loading='lazy' decoding='async' />}
              </Show>

              <div class='flex min-w-0 flex-1 flex-col gap-0'>
                <p class='song-item-title archivo text-md truncate font-bold' title={entry.title}>
                  {entry.title}
                </p>
                <p class='song-item-meta archivo truncate text-[11px]' title={entry.artist ?? '[unknown artist]'}>
                  {entry.artist}
                </p>
              </div>

              <p class='song-item-meta archivo ml-2 text-xs'>{formatCompactDuration(entry.duration ?? 0)}</p>
            </div>
          );
        }}
      </For>
    </div>
  );
};

export default QueueList;
