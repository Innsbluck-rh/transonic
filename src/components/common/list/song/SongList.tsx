import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import { resolveSongItemConditions, songItemConditionAttrs } from '~/features/items/SongItemConditions';
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
          const conditions = () =>
            resolveSongItemConditions({
              playbackStatus: playbackStore.status,
              songId: song.id,
            });

          return (
            <div
              class='song-item ripple flex w-full cursor-pointer flex-row items-center overflow-x-hidden px-4 py-3'
              onClick={onClickEntry}
              {...songItemConditionAttrs(conditions())}
              {...contextMenuProps}
            >
              <div class='flex w-10 flex-row items-start'>
                <Show
                  when={!conditions().current}
                  fallback={<Icon class='song-item-leading -ml-0.5 w-fit scale-125 text-xs' icon='material-symbols:play-arrow' />}
                >
                  <p class='song-item-leading archivo text-xs font-bold'>{song.track || '?'}</p>
                </Show>
              </div>

              <p class='song-item-title min-w-0 flex-1 truncate text-sm' title={song.title}>
                {song.title}
              </p>

              <p class='song-item-meta archivo text-xs'>{formatCompactDuration(song.duration ?? 0)}</p>
            </div>
          );
        }}
      </For>
    </div>
  );
};

export default SongList;
