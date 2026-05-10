import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
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

          const isPlaying = () => playbackStore.status?.currentSongId === entry.id && playbackStore.status?.currentIndex === i();

          return (
            <div
              ref={(element) => props.itemRef?.(i(), element)}
              class='group ripple hover:bg-primary-hover flex h-auto cursor-pointer flex-row items-center overflow-x-hidden px-4 py-2.5'
              classList={{
                'bg-primary-playing': isPlaying(),
              }}
              onClick={onClickEntry}
              {...contextMenuProps}
            >
              <div class='group-hover:text-primary-text flex w-7 flex-row items-start'>
                <Show
                  when={!isPlaying()}
                  fallback={
                    <Icon
                      class='group-hover:text-primary-text text-primary-on-playing -ml-0.5 w-fit scale-125 text-xs'
                      icon='material-symbols:play-arrow'
                    />
                  }
                >
                  <p class='group-hover:text-primary-text archivo text-xs font-bold'>{i() + 1}</p>
                </Show>
              </div>

              <Show when={coverArtMap.src(entry.coverArtId)} fallback={<div class='mr-3 aspect-square max-h-8 w-8 rounded-md' />}>
                {(assetUrl) => <img src={assetUrl()} class='mr-3 aspect-square max-h-8 w-8 rounded-md' loading='lazy' decoding='async' />}
              </Show>

              <div class='flex min-w-0 flex-1 flex-col gap-1'>
                <p
                  class='group-hover:text-primary-text truncate text-sm leading-none'
                  classList={{
                    'text-primary-on-playing font-bold': isPlaying(),
                  }}
                  title={entry.title}
                >
                  {entry.title}
                </p>
                <p
                  class='group-hover:text-secondary-text truncate text-xs leading-none'
                  classList={{
                    'text-secondary-text': !isPlaying(),
                    'text-primary-on-playing': isPlaying(),
                  }}
                  title={entry.artist ?? '[unknown artist]'}
                >
                  {entry.artist}
                </p>
              </div>

              <p
                class='archivo ml-2 text-xs'
                classList={{
                  'text-secondary-text': !isPlaying(),
                  'text-primary-on-playing': isPlaying(),
                }}
              >
                {formatCompactDuration(entry.duration ?? 0)}
              </p>
            </div>
          );
        }}
      </For>
    </div>
  );
};

export default QueueList;
