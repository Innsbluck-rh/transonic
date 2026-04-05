import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import { playbackStore } from '~/stores/PlaybackStore';

interface SongListProps {
  songs?: SongResponse[];
  onClickSong?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
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

const SongList: Component<SongListProps> = (props) => {
  return (
    <div class='flex min-h-0 w-full flex-1 flex-col overflow-x-hidden overflow-y-auto'>
      <For
        each={props.songs}
        fallback={
          <div class='col-span-2 flex w-full p-2'>
            <p class='text-secondary-text px-1 text-xs italic'>[no songs]</p>
          </div>
        }
      >
        {(song, i) => {
          const onClickEntry = () => {
            if (!props.songs) return;
            props.onClickSong?.(song, i(), props.songs);

            commands.playbackPlaySongs({
              songs: props.songs,
              startIndex: i(),
            });
          };

          const isPlaying = () => playbackStore.status?.currentSongId === song.id;

          return (
            <div
              class='group ripple hover:bg-primary-hover flex w-full cursor-pointer flex-row items-center overflow-x-hidden px-4 py-3'
              classList={{
                'bg-primary-playing': isPlaying(),
              }}
              onClick={onClickEntry}
            >
              <div class='group-hover:text-primary-text flex w-10 flex-row items-start'>
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

              <p
                class='group-hover:text-primary-text min-w-0 flex-1 truncate text-sm'
                classList={{
                  'text-primary-on-playing font-bold': isPlaying(),
                }}
                title={song.title}
              >
                {song.title}
              </p>

              <p
                class='archivo text-xs'
                classList={{
                  'text-secondary-text': !isPlaying(),
                  'text-primary-on-playing': isPlaying(),
                }}
              >
                {formatDurationSecond(song.duration ?? 0)}
              </p>
            </div>
          );
        }}
      </For>
    </div>
  );
};

export default SongList;
