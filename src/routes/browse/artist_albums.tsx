import { useParams } from '@solidjs/router';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import Heading2 from '~/components/common/Heading2';
import AlbumGrid from '~/components/list/AlbumGrid';
import { fetchMusicDirectory } from '~/features/browse/service';
import { AlbumListItem } from '~/models/album';
import { MusicDirectoryResponse } from '~/models/browse';
import { sessionStore } from '~/stores/session/SessionStore';

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
        name: child.name,
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
      const response = await fetchMusicDirectory({ id: artistId });
      setDirectory(response);
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
    <div class='flex flex-col gap-4 p-3 h-full w-full overflow-x-hidden overflow-y-auto bg-zinc-100'>
      <Heading2>{directory()?.name ?? '[Unknown]'}</Heading2>

      <Show when={error()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<p class='text-sm text-zinc-400'>Loading artist albums...</p>}>
        <AlbumGrid albums={albums()} emptyMessage={emptyMessage()} />
      </Show>
    </div>
  );
}

export default BrowseArtistAlbums;
