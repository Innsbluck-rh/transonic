import { Component, For } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import SongItem from '~/components/common/list/song/SongItem';
import { buildSongMenuItems } from '~/features/menu';
import { isCurrentSong } from '~/features/playback/currentSong';
import { usePlayback } from '~/features/playback/usePlayback';

interface AlbumTrackListProps {
  songs?: SongResponse[];
  onClickSong?: (song: SongResponse, index: number, songs: SongResponse[]) => void;
}

const AlbumTrackList: Component<AlbumTrackListProps> = (props) => {
  const { status: playbackStatus } = usePlayback();

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
          return (
            <SongItem
              song={song}
              playing={isCurrentSong({ playbackStatus: playbackStatus(), songId: song.id })}
              index={song.track ?? undefined}
              contextMenuItems={buildSongMenuItems(song)}
              onClick={onClickEntry}
            />
          );
        }}
      </For>
    </div>
  );
};

export default AlbumTrackList;
