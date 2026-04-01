import { useParams } from '@solidjs/router';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import { commands, PlaybackQueueEntry, type AlbumListItem, type MusicDirectoryResponse } from '~/bindings';
import Heading3 from '~/components/common/Heading3';
import LoadCircle from '~/components/common/LoadCircle';
import AlbumGrid from '~/components/common/list/AlbumGrid';
import { hasPlaybackCommandError } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';

function BrowseArtistAlbums() {
  const params = useParams();

  const [directory, setDirectory] = createSignal<MusicDirectoryResponse | null>(null);
  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const albums = createMemo<AlbumListItem[]>(() =>
    (directory()?.children ?? [])
      .filter((child) => child.isDirectory)
      .map((child) => ({
        id: child.id,
        name: child.title,
        artist: child.artist,
        coverArtId: child.coverArtId,
        year: child.year,
      }))
  );

  const emptyMessage = createMemo(() => {
    const children = directory()?.children ?? [];
    if (children.length > 0 && albums().length === 0) {
      return 'Direct song artist browse is not supported yet.';
    }

    return 'No albums available.';
  });

  async function loadArtistDirectory(rawArtistId: string) {
    if (!sessionStore.activeSession) {
      setDirectory(null);
      setError(null);
      setIsLoading(false);
      return;
    }

    const artistId = decodeURIComponent(rawArtistId || '').trim();
    if (!artistId) {
      setDirectory(null);
      setError('Artist id is missing.');
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const result = await commands.getMusicDirectory({ id: artistId });
      if (result.status === 'error') {
        setDirectory(null);
        setError(result.error);
        return;
      }

      setDirectory(result.data);
    } catch (invokeError) {
      console.error(invokeError);
      setDirectory(null);
      setError('Failed to load albums for the selected artist.');
    } finally {
      setIsLoading(false);
    }
  }

  createEffect(() => {
    void sessionStore.activeSession;
    void loadArtistDirectory(params.id || '');
  });

  return (
    <div class='home-surface-root'>
      <Show when={error()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<LoadCircle class='self-center justify-self-center' />}>
        <Heading3>artists/{directory()?.name ?? '[Unknown]'}</Heading3>
        <AlbumGrid
          albums={albums()}
          emptyMessage={emptyMessage()}
          onClickItem={async (album) => {
            // get album songs
            const asResult = await commands.getAlbumSongs({ id: album.id });
            if (asResult.status == 'error') {
              return;
            }
            const entries: PlaybackQueueEntry[] = asResult.data.songs.map((song) => {
              return { songId: song.id } as PlaybackQueueEntry;
            });
            // TODO: consider batch this?
            const setQueueResult = await commands.playbackSetQueue({ currentIndex: 0, entries });
            if (hasPlaybackCommandError(setQueueResult)) {
              return;
            }
            const playResult = await commands.playbackPlay();
            hasPlaybackCommandError(playResult);
          }}
        />
      </Show>
    </div>
  );
}

export default BrowseArtistAlbums;
