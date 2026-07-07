import { Icon } from '@iconify-icon/solid';
import { Component, For } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import SongItem from '~/components/common/list/song/SongItem';
import { resolveSongItemConditions } from '~/features/items/SongItemConditions';
import { buildSongMenuItems } from '~/features/menu';
import { usePlayback } from '~/features/playback/usePlayback';

interface AlbumTrackListProps {
  songs?: SongResponse[];
  onClickSong?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
}

const AlbumTrackList: Component<AlbumTrackListProps> = (props) => {
  const { insertAfterCurrent, appendToQueue, status: playbackStatus } = usePlayback();

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

            commands.playbackSetQueue({
              items: [{ type: 'songs', songs: props.songs }],
              currentIndex: i(),
              autoPlay: true,
            });
          };
          const conditions = () =>
            resolveSongItemConditions({
              playbackStatus: playbackStatus(),
              songId: song.id,
            });

          return (
            <SongItem
              song={song}
              conditions={conditions()}
              leadingContent={
                conditions().current ? (
                  <Icon class='song-item-leading -ml-0.5 w-fit scale-125 text-xs' icon='material-symbols:play-arrow' />
                ) : song.track != null ? (
                  String(song.track)
                ) : (
                  '?'
                )
              }
              menuItems={buildSongMenuItems(song, { insertAfterCurrent, appendToQueue })}
              onClick={onClickEntry}
            />
          );
        }}
      </For>
    </div>
  );
};

export default AlbumTrackList;
