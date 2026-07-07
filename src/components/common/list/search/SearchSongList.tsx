import { Component, For } from 'solid-js';
import { commands, type SongResponse } from '~/bindings';
import SongItem from '~/components/common/list/song/SongItem';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
import { resolveSongItemConditions } from '~/features/items/SongItemConditions';
import { buildSongMenuItems } from '~/features/menu';
import { usePlayback } from '~/features/playback/usePlayback';

interface SearchSongListProps {
  songs?: SongResponse[];
}

/**
 * @description 検索画面の曲一行リスト（カバーアート付き・番号列なし）
 */
const SearchSongList: Component<SearchSongListProps> = (props) => {
  const { insertAfterCurrent, appendToQueue, status: playbackStatus } = usePlayback();
  const coverArtMap = useCoverArtMap(() => (props.songs ?? []).map((song) => song.displayCoverArtId ?? song.coverArtId), CoverArtSizes.sm, {
    cachedFallbackSizes: [CoverArtSizes.lg, CoverArtSizes.md],
  });

  return (
    <For each={props.songs}>
      {(song, i) => {
        const conditions = () => resolveSongItemConditions({ playbackStatus: playbackStatus(), songId: song.id });
        return (
          <SongItem
            song={song}
            conditions={conditions()}
            coverArt
            coverArtUrl={coverArtMap.src(song.displayCoverArtId ?? song.coverArtId)}
            menuItems={buildSongMenuItems(song, { insertAfterCurrent, appendToQueue })}
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
