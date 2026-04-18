import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import { buildSongMenuItems } from '~/features/menu';
import useContextMenu from '~/features/menu/useContextMenu';
import { usePlayback } from '~/features/playback/usePlayback';
import { playbackStore } from '~/stores/PlaybackStore';
import { formatCompactDuration } from '~/utils/duration';

interface SongListProps {
  songs?: SongResponse[];
  onClickSong?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
}

const SongList: Component<SongListProps> = (props) => {
  const { insertAfterCurrent, appendToQueue } = usePlayback();

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
          const contextMenuProps = useContextMenu(buildSongMenuItems(song, { insertAfterCurrent, appendToQueue }));

          const onClickEntry = () => {
            if (!props.songs) return;
            props.onClickSong?.(song, i(), props.songs);

            commands.playbackSetQueue({
              items: [{ type: 'songs', songs: props.songs }],
              currentIndex: i(),
              autoPlay: true,
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
              {...contextMenuProps}
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
                class='archivo group-hover:text-secondary-text text-xs'
                classList={{
                  'text-secondary-text': !isPlaying(),
                  'text-primary-on-playing': isPlaying(),
                }}
              >
                {formatCompactDuration(song.duration ?? 0)}
              </p>
            </div>
          );
        }}
      </For>
    </div>
  );
};

export default SongList;
