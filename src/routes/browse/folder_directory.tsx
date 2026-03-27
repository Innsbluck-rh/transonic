import { useNavigate, useParams } from '@solidjs/router';
import { createEffect, createSignal, Show } from 'solid-js';
import Heading2 from '~/components/common/Heading2';
import BrowseList from '~/components/list/BrowseList';
import MusicDirectoryList from '~/components/list/MusicDirectoryList';
import { fetchArtistIndexes, fetchMusicDirectory, fetchMusicFolders } from '~/features/browse/service';
import { ArtistIndexItem, MusicDirectoryResponse } from '~/models/browse';
import { sessionStore } from '~/stores/session/SessionStore';

type FolderBrowseView = 'directory' | 'music_folder' | null;

function readInvokeErrorMessage(invokeError: unknown) {
  if (invokeError instanceof Error) {
    return invokeError.message;
  }

  if (typeof invokeError === 'string') {
    return invokeError;
  }

  return String(invokeError);
}

function isDirectoryNotFoundError(message: string) {
  return message.includes('Directory not found') || message.includes('API 70');
}

function BrowseFolderDirectory() {
  const navigate = useNavigate();
  const params = useParams();

  const [directory, setDirectory] = createSignal<MusicDirectoryResponse | null>(null);
  const [folderArtists, setFolderArtists] = createSignal<ArtistIndexItem[]>([]);
  const [folderName, setFolderName] = createSignal('[Unknown]');
  const [viewKind, setViewKind] = createSignal<FolderBrowseView>(null);
  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  async function loadMusicFolderArtists(musicFolderId: string) {
    const [foldersResponse, artistsResponse] = await Promise.all([fetchMusicFolders(), fetchArtistIndexes({ musicFolderId })]);
    const matchedFolder = foldersResponse.musicFolders.find((folder) => folder.id === musicFolderId);

    if (!matchedFolder) {
      throw new Error(`Music folder not found: ${musicFolderId}`);
    }

    setViewKind('music_folder');
    setDirectory(null);
    setFolderArtists(artistsResponse.artists);
    setFolderName(matchedFolder.name);
  }

  async function loadDirectory(rawDirectoryId: string) {
    if (!sessionStore.activeSession) {
      setDirectory(null);
      setFolderArtists([]);
      setFolderName('[Unknown]');
      setViewKind(null);
      setError(null);
      setIsLoading(false);
      return;
    }

    const directoryId = decodeURIComponent(rawDirectoryId || '').trim();
    if (!directoryId) {
      setDirectory(null);
      setFolderArtists([]);
      setFolderName('[Unknown]');
      setViewKind(null);
      setError('Directory id is missing.');
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetchMusicDirectory({ id: directoryId });
      setViewKind('directory');
      setDirectory(response);
      setFolderArtists([]);
      setFolderName(response.name);
    } catch (invokeError) {
      const errorMessage = readInvokeErrorMessage(invokeError);

      if (isDirectoryNotFoundError(errorMessage)) {
        try {
          await loadMusicFolderArtists(directoryId);
          return;
        } catch (fallbackError) {
          console.error(fallbackError);
        }
      }

      console.error(invokeError);
      setDirectory(null);
      setFolderArtists([]);
      setFolderName('[Unknown]');
      setViewKind(null);
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }

  createEffect(() => {
    void sessionStore.activeSession;
    void loadDirectory(params.id || '');
  });

  return (
    <div class='flex flex-col gap-4 p-3 h-full w-full overflow-x-hidden overflow-y-auto bg-zinc-100'>
      <Heading2>{viewKind() === 'directory' ? (directory()?.name ?? folderName()) : folderName()}</Heading2>

      <Show when={error()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<p class='text-sm text-zinc-400'>Loading directory contents...</p>}>
        <Show
          when={viewKind() === 'music_folder'}
          fallback={
            <MusicDirectoryList
              items={directory()?.children ?? []}
              emptyMessage='No items available in this directory.'
              onClickDirectory={(item) => {
                navigate(`/browse/folders/${encodeURIComponent(item.id)}`);
              }}
            />
          }
        >
          <BrowseList
            items={folderArtists()}
            isLoading={false}
            error={null}
            emptyMessage='No artists available in this music folder.'
            onClickItem={(item) => {
              navigate(`/browse/folders/${encodeURIComponent(item.id)}`);
            }}
          />
        </Show>
      </Show>
    </div>
  );
}

export default BrowseFolderDirectory;
