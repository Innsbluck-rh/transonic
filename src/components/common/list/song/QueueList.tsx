import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { commands, PlaybackQueueEntry } from '~/bindings';
import { playbackStore } from '~/stores/PlaybackStore';

interface QueueListProps {
  queue: PlaybackQueueEntry[];
  onClickQueue?: (song: PlaybackQueueEntry, index: number, songs: PlaybackQueueEntry[]) => void;
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

// TODO: you know its VERY similar to SongList.tsx but it's not compatible because of poor(old) implementation in Queue item.
// consider replace PlaybackQueueEntry to Just Raw Song data and merge this to SongList(and Stop hardcoding play function in songlist itself...)
const QueueList: Component<QueueListProps> = (props) => {
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
          const onClickEntry = async () => {
            if (!props.queue) return;
            props.onClickQueue?.(entry, i(), props.queue);

            await commands.playbackPlayQueueIndex({
              index: i(),
            });
          };

          const isPlaying = () => playbackStore.status?.currentSongId === entry.songId; // ← here is cause of incompatibility fxxx

          return (
            <div
              class='group ripple hover:bg-primary-hover flex h-auto cursor-pointer flex-row items-center overflow-x-hidden px-4 py-3'
              classList={{
                'bg-primary-playing': isPlaying(),
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

              <p
                class='group-hover:text-primary-text min-w-0 flex-1 truncate text-sm'
                classList={{
                  'text-primary-on-playing font-bold': isPlaying(),
                }}
                title={entry.title}
              >
                {entry.title}
              </p>

              <p class='text-secondary-text archivo ml-2 text-xs'>{formatDurationSecond(entry.duration ?? 0)}</p>
            </div>
          );
        }}
      </For>
    </div>
  );
};

export default QueueList;
