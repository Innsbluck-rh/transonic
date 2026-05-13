import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
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

const QueueList: Component<QueueListProps> = (props) => {
  const navigate = useSPNavigate();

  const coverArtMap = useCoverArtMap(() => props.queue.map((song) => song.coverArtId), 48);

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
              ref={(element) => props.itemRef?.(i(), element)}
              class='song-item ripple flex h-auto cursor-pointer flex-row items-center overflow-x-hidden px-4 py-2.5'
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

              <Show when={coverArtMap.src(entry.coverArtId)} fallback={<div class='mr-3 aspect-square max-h-8 w-8 rounded-md' />}>
                {(assetUrl) => <img src={assetUrl()} class='mr-3 aspect-square max-h-8 w-8 rounded-md' loading='lazy' decoding='async' />}
              </Show>

              <div class='flex min-w-0 flex-1 flex-col gap-1'>
                <p class='song-item-title truncate text-sm leading-none' title={entry.title}>
                  {entry.title}
                </p>
                <p class='song-item-meta truncate text-xs leading-none' title={entry.artist ?? '[unknown artist]'}>
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
