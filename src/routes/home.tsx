import { useNavigate } from '@solidjs/router';
import { invoke } from '@tauri-apps/api/core';
import { createSignal, For, onMount, Show } from 'solid-js';
import Heading1 from '~/components/common/Heading1';
import Header from '~/components/header/Header';
import AlbumHorizontalList from '~/components/list/AlbumHorizontalList';
import { fetchAlbumList } from '~/features/albums/service';
import { loadBootstrapToStore } from '~/features/session/service';
import { AlbumListContext, AlbumListItem } from '~/models/album';
import { AppBootstrap } from '~/models/bootstrap';
import { sessionStore } from '~/stores/session/SessionStore';

type HomeAlbumSection = {
  heading: string;
  context: AlbumListContext;
  albums: AlbumListItem[];
};

const HOME_ALBUM_CONTEXTS: Array<Pick<HomeAlbumSection, 'heading' | 'context'>> = [
  { heading: 'newest', context: 'newest' },
  { heading: 'random picks', context: 'random' },
];

function Home() {
  const navigate = useNavigate();
  const [albumSections, setAlbumSections] = createSignal<HomeAlbumSection[]>([]);
  const [isLoadingAlbums, setIsLoadingAlbums] = createSignal(false);
  const [albumError, setAlbumError] = createSignal<string | null>(null);

  async function loadHomeAlbumSections() {
    setIsLoadingAlbums(true);
    setAlbumError(null);

    try {
      const sections = await Promise.all(
        HOME_ALBUM_CONTEXTS.map(async (section) => {
          const response = await fetchAlbumList({
            context: section.context,
            size: 8,
          });

          return {
            ...section,
            albums: response.albums,
          };
        })
      );

      setAlbumSections(sections);
    } catch (invokeError) {
      console.error(invokeError);
      setAlbumError('Failed to load albums from the active server.');
      setAlbumSections([]);
    } finally {
      setIsLoadingAlbums(false);
    }
  }

  onMount(async () => {
    try {
      const bootstrap = await invoke<AppBootstrap>('bootstrap_app_state');
      loadBootstrapToStore(bootstrap);

      switch (bootstrap.restoreStatus) {
        case 'restored':
          break;
        case 'none':
          if (!bootstrap.activeSession) {
            navigate('/init_login');
            return;
          }
          break;
        case 'offline':
        case 'reauth_required':
          navigate('/init_login');
          return;
      }

      if (bootstrap.activeSession) {
        await loadHomeAlbumSections();
      }
    } catch (invokeError) {
      console.error(invokeError);
      setAlbumError('Failed to load the active session.');
    }
  });

  return (
    <Show
      when={sessionStore.activeSession}
      fallback={
        <div>
          <p>seems like there's no active profile.</p>
        </div>
      }
    >
      <div class='flex flex-col gap-1.5'>
        <Header shouldShowProfiles={true} />

        <div class='flex flex-col gap-4 p-3'>
          <Show when={albumError()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

          <Show when={!isLoadingAlbums()} fallback={<p class='text-sm text-zinc-400'>Loading album lists...</p>}>
            <For each={albumSections()}>
              {(section) => (
                <section class='flex flex-col gap-1.5'>
                  <Heading1>{section.heading}</Heading1>
                  <AlbumHorizontalList albums={section.albums} emptyMessage={`No albums returned for ${section.heading}.`} />
                </section>
              )}
            </For>
          </Show>
        </div>
      </div>
    </Show>
  );
}

export default Home;
