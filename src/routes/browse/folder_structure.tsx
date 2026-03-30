import { useParams } from '@solidjs/router';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import type { AlbumListItem, FolderStructureAlbumsResponse, FolderStructureRootsResponse } from '~/bindings';
import Heading2 from '~/components/common/Heading2';
import AlbumGrid from '~/components/list/AlbumGrid';
import { fetchFolderStructureAlbums, fetchFolderStructureRoots, setFolderStructureSelectedLibraryId } from '~/features/browse/folderStructureService';
import { sessionStore } from '~/stores/session/SessionStore';

function decodePathParam(value?: string) {
  if (!value) {
    return null;
  }

  try {
    const trimmed = decodeURIComponent(value).trim();
    return trimmed.length > 0 ? trimmed : null;
  } catch (_error) {
    return value;
  }
}

function readInvokeErrorMessage(invokeError: unknown) {
  if (invokeError instanceof Error) {
    return invokeError.message;
  }

  if (typeof invokeError === 'string') {
    return invokeError;
  }

  return String(invokeError);
}

function BrowseFolderStructure() {
  const params = useParams();

  const [rootsResponse, setRootsResponse] = createSignal<FolderStructureRootsResponse | null>(null);
  const [albumsResponse, setAlbumsResponse] = createSignal<FolderStructureAlbumsResponse | null>(null);
  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const libraryId = createMemo(() => decodePathParam(params.libraryId));
  const nodeId = createMemo(() => decodePathParam(params.nodeId));
  const albums = createMemo<AlbumListItem[]>(() =>
    (albumsResponse()?.albums ?? []).map((album) => ({
      id: album.id,
      name: album.name,
      artist: album.artist,
      coverArtId: album.coverArtId,
      year: album.year,
    }))
  );
  const heading = createMemo(() => albumsResponse()?.nodeName ?? rootsResponse()?.libraryName ?? 'Folder Structure');

  async function loadFolderStructureView() {
    if (!sessionStore.activeSession) {
      setRootsResponse(null);
      setAlbumsResponse(null);
      setError(null);
      setIsLoading(false);
      return;
    }

    const nextLibraryId = libraryId();
    if (!nextLibraryId) {
      setRootsResponse(null);
      setAlbumsResponse(null);
      setError('Library id is missing.');
      setIsLoading(false);
      return;
    }

    setFolderStructureSelectedLibraryId(nextLibraryId);
    setIsLoading(true);
    setError(null);

    try {
      const nextRootsResponse = await fetchFolderStructureRoots({ libraryId: nextLibraryId });
      setRootsResponse(nextRootsResponse);

      const nextNodeId = nodeId();
      if (!nextNodeId) {
        setAlbumsResponse(null);
        return;
      }

      const nextAlbumsResponse = await fetchFolderStructureAlbums({
        libraryId: nextLibraryId,
        nodeId: nextNodeId,
      });
      setAlbumsResponse(nextAlbumsResponse);
    } catch (invokeError) {
      console.error(invokeError);
      setRootsResponse(null);
      setAlbumsResponse(null);
      setError(readInvokeErrorMessage(invokeError));
    } finally {
      setIsLoading(false);
    }
  }

  createEffect(() => {
    void sessionStore.activeSession;
    void params.libraryId;
    void params.nodeId;
    void loadFolderStructureView();
  });

  return (
    <div class='flex flex-col gap-4 p-3 h-full w-full overflow-x-hidden overflow-y-auto bg-zinc-100'>
      <Heading2>{heading()}</Heading2>

      <Show when={error()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<p class='text-sm text-zinc-400'>Loading folder structure...</p>}>
        <Show when={nodeId()} fallback={<p class='text-sm text-zinc-500'>Select a folder.</p>}>
          <AlbumGrid albums={albums()} emptyMessage='No albums available in this folder.' />
        </Show>
      </Show>
    </div>
  );
}

export default BrowseFolderStructure;
