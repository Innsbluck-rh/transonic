import { useNavigate } from '@solidjs/router';
import { createEffect, createSignal, Show } from 'solid-js';
import Heading2 from '~/components/common/Heading2';
import { setFolderStructureSelectedLibraryId } from '~/features/browse/folderStructureService';
import { fetchMusicFolders } from '~/features/browse/service';
import { sessionStore } from '~/stores/session/SessionStore';

function BrowseFolders() {
  const navigate = useNavigate();

  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [libraryCount, setLibraryCount] = createSignal(0);

  async function loadLibraries() {
    if (!sessionStore.activeSession) {
      setLibraryCount(0);
      setError(null);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetchMusicFolders();
      setLibraryCount(response.musicFolders.length);

      if (response.musicFolders.length === 1) {
        const libraryId = response.musicFolders[0].id;
        setFolderStructureSelectedLibraryId(libraryId);
        navigate(`/browse/folders/${encodeURIComponent(libraryId)}`, { replace: true });
      }
    } catch (invokeError) {
      console.error(invokeError);
      setLibraryCount(0);
      setError('Failed to load libraries.');
    } finally {
      setIsLoading(false);
    }
  }

  createEffect(() => {
    void sessionStore.activeSession;
    void loadLibraries();
  });

  return (
    <div class='flex flex-col gap-4 p-3 h-full w-full overflow-x-hidden overflow-y-auto bg-zinc-100'>
      <Heading2>Folder Structure</Heading2>

      <Show when={error()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<p class='text-sm text-zinc-400'>Loading libraries...</p>}>
        <Show
          when={libraryCount() > 1}
          fallback={<p class='text-sm text-zinc-500'>{libraryCount() === 0 ? 'No libraries available.' : 'Redirecting to the only library...'}</p>}
        >
          <p class='text-sm text-zinc-500'>Select a library.</p>
        </Show>
      </Show>
    </div>
  );
}

export default BrowseFolders;
