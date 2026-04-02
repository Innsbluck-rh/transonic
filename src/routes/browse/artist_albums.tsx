import { useParams } from '@solidjs/router';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import { commands, type AlbumListItem, type MusicDirectoryResponse } from '~/bindings';
import LoadCircle from '~/components/common/LoadCircle';
import AlbumGrid from '~/components/common/list/album/AlbumGrid';
import { replaceQueueWithAlbumAndPlay } from '~/features/playback/album';
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
    <div class='home-surface-root p-3'>
      <Show when={error()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<LoadCircle class='self-center justify-self-center' />}>
        {/* <Heading3>artists/{directory()?.name ?? '[Unknown]'}</Heading3> */}
        <AlbumGrid
          albums={albums()}
          emptyMessage={emptyMessage()}
          onItemClick={async (album) => {
            replaceQueueWithAlbumAndPlay(album.id);
          }}
        />
      </Show>
    </div>
  );
}

export default BrowseArtistAlbums;
