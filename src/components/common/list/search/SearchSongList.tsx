import { Component, For } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import SongItem from '~/components/common/list/song/SongItem';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
import { buildSongMenuItems } from '~/features/menu';
import { isCurrentSong } from '~/features/playback/currentSong';
import { usePlayback } from '~/features/playback/usePlayback';

interface SearchSongListProps {
  songs?: SongResponse[];
}

/**
 * @description 検索画面の曲一行リスト（カバーアート付き・番号列なし）
 */
const SearchSongList: Component<SearchSongListProps> = (props) => {
  const { status: playbackStatus } = usePlayback();
  const coverArtMap = useCoverArtMap(() => (props.songs ?? []).map((song) => song.displayCoverArtId ?? song.coverArtId), CoverArtSizes.sm, {
    cachedFallbackSizes: [CoverArtSizes.lg, CoverArtSizes.md],
  });

  return (
    <For each={props.songs}>
      {(song, i) => {
        return (
          <SongItem
            song={song}
            playing={isCurrentSong({ playbackStatus: playbackStatus(), songId: song.id })}
            indexContentMode='hidden'
            coverArt
            coverArtUrl={coverArtMap.src(song.displayCoverArtId ?? song.coverArtId)}
            contextMenuItems={buildSongMenuItems(song)}
            onClick={() => {
              const songs = props.songs ?? [];
              if (songs.length === 0) return;
              commands.playbackSetQueue({ items: [{ type: 'songs', songs }], currentIndex: i(), autoPlay: true });
            }}
          />
        );
      }}
    </For>
  );
};

export default SearchSongList;
