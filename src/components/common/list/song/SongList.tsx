import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import { playbackStore } from '~/stores/PlaybackStore';

interface SongListProps {
  songs?: SongResponse[];
  onClickSong?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
}

const SongList: Component<SongListProps> = (props) => {
  return (
    <div class='flex min-h-0 flex-1 overflow-x-hidden overflow-y-auto'>
      <div class='grid h-fit w-full grid-cols-[max-content_minmax(0,1fr)]'>
        <For
          each={props.songs}
          fallback={
            <div class='col-span-2 flex w-full p-2'>
              <p class='text-secondary-text px-1 text-xs italic'>[no songs]</p>
            </div>
          }
        >
          {(song, i) => {
            const onClickEntry = async () => {
              if (!props.songs) return;
              props.onClickSong?.(song, i(), props.songs);

              await commands.playbackPlaySongs({
                songs: props.songs,
                startIndex: i(),
              });
            };

            const isPlaying = () => playbackStore.status?.currentSongId === song.id;

            return (
              <div
                class='ripple hover:bg-primary-hover col-span-2 grid w-full cursor-pointer grid-cols-subgrid items-center gap-x-3 px-3 py-3'
                classList={{
                  'bg-primary-playing': isPlaying(),
                }}
                onClick={onClickEntry}
              >
                <div class='flex flex-row items-start'>
                  <Show
                    when={!isPlaying()}
                    fallback={<Icon class='text-accent -ml-0.5 w-fit scale-125 text-xs' icon='material-symbols:play-arrow' />}
                  >
                    <p class='archivo text-xs font-bold'>{i() + 1}</p>
                  </Show>
                </div>

                <p
                  class='min-w-0 truncate text-xs'
                  classList={{
                    'text-accent font-bold': isPlaying(),
                  }}
                  title={song.title}
                >
                  {song.title}
                </p>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};

export default SongList;
