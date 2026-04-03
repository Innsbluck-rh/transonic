import { useParams } from '@solidjs/router';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import { commands, type FolderStructureAlbumsResponse, type FolderStructureRootsResponse } from '~/bindings';
import LoadCircle from '~/components/common/LoadCircle';
import AlbumGrid from '~/components/common/list/album/AlbumGrid';
import { AlbumItem } from '~/components/common/list/album/item/AlbumGridItem';
import { usePlayback } from '~/features/playback/usePlayback';
import { sessionStore } from '~/stores/SessionStore';

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
  const { playFolderAlbum } = usePlayback();

  const [rootsResponse, setRootsResponse] = createSignal<FolderStructureRootsResponse | null>(null);
  const [albumsResponse, setAlbumsResponse] = createSignal<FolderStructureAlbumsResponse | null>(null);
  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const libraryId = createMemo(() => decodePathParam(params.libraryId));
  const nodeId = createMemo(() => decodePathParam(params.nodeId));
  const albums = createMemo<AlbumItem[]>(() => albumsResponse()?.albums ?? []);
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

    setIsLoading(true);
    setError(null);

    try {
      const nextNodeId = nodeId();
      if (!nextNodeId) {
        const nextFsrResult = await commands.getFolderStructureRoots({ libraryId: nextLibraryId });
        if (nextFsrResult.status === 'error') {
          throw new Error(nextFsrResult.error);
        }

        setRootsResponse(nextFsrResult.data);
        setAlbumsResponse(null);
        setIsLoading(false);
        return;
      }

      const [nextFsrResult, nextFsaResult] = await Promise.all([
        commands.getFolderStructureRoots({ libraryId: nextLibraryId }),
        commands.getFolderStructureAlbums({
          libraryId: nextLibraryId,
          nodeId: nextNodeId,
        }),
      ]);

      if (nextFsrResult.status === 'error') {
        throw new Error(nextFsrResult.error);
      }
      if (nextFsaResult.status === 'error') {
        throw new Error(nextFsaResult.error);
      }

      setRootsResponse(nextFsrResult.data);
      setAlbumsResponse(nextFsaResult.data);
    } catch (invokeError) {
      console.error(invokeError);
      setRootsResponse(null);
      setAlbumsResponse(null);
      setError(readInvokeErrorMessage(invokeError));
      setIsLoading(false);
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
    <div class='home-surface-root p-3'>
      <Show when={error()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<LoadCircle class='self-center justify-self-center' />}>
        {/* <Heading3>folders/{heading()}</Heading3> */}
        <Show when={nodeId()} fallback={<p class='text-sm text-zinc-500'>Select a folder.</p>}>
          <AlbumGrid
            albums={albums()}
            emptyMessage='No albums available in this folder.'
            onItemClick={async (album) => {
              if (!params.libraryId || !params.nodeId) return;
              await playFolderAlbum(params.libraryId, params.nodeId, album.id);
            }}
          />
        </Show>
      </Show>
    </div>
  );
}

export default BrowseFolderStructure;
