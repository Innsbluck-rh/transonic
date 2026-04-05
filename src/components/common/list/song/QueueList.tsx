import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import { buildQueueItemMenuItems, showContextMenu } from '~/features/menu';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { playbackStore } from '~/stores/PlaybackStore';

interface QueueListProps {
  queue: SongResponse[];
  onClickQueue?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
}

// consider utilize this
function formatDurationSecond(duration: number) {
  const h = Math.round(duration / 3600);
  const m = Math.round(duration / 60) - h * 60;
  const s = Math.round(duration % 60);
  if (h > 0) {
    return `${String(h)}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  } else if (m > 0) {
    return `${String(m)}:${String(s).padStart(2, '0')}`;
  } else if (s > 0) {
    return `0:${String(s).padStart(2, '0')}`;
  } else {
    return '';
  }
}

const QueueList: Component<QueueListProps> = (props) => {
  const navigate = useSPNavigate();

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

          const isPlaying = () => playbackStore.status?.currentSongId === entry.id && playbackStore.status?.currentIndex === i();

          return (
            <div
              class='group ripple hover:bg-primary-hover flex h-auto cursor-pointer flex-row items-center overflow-x-hidden px-4 py-2.5'
              classList={{
                'bg-primary-playing': isPlaying(),
              }}
              onContextMenu={(e) => {
                showContextMenu(e, buildQueueItemMenuItems(entry, i(), navigate));
              }}
              onClick={onClickEntry}
            >
              <div class='group-hover:text-primary-text flex w-8 flex-row items-start'>
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

              {/* <img src={coverArts[i()] || ''} class='mr-3 aspect-square max-h-8 w-8 rounded-md' /> */}

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
                {formatDurationSecond(entry.duration ?? 0)}
              </p>
            </div>
          );
        }}
      </For>
    </div>
  );
};

export default QueueList;
