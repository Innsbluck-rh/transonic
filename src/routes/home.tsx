import { createSignal, For, onMount, Show } from 'solid-js';
import Heading1 from '~/components/common/Heading1';
import AlbumHorizontalList from '~/components/list/AlbumHorizontalList';
import { fetchAlbumList } from '~/features/albums/service';
import { AlbumListContext, AlbumListItem } from '~/models/album';
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
    await loadHomeAlbumSections();
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
      <div class='flex flex-col gap-4 p-3 w-full h-full bg-zinc-100 overflow-x-hidden overflow-y-auto'>
        <Show when={albumError()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

        <Show when={!isLoadingAlbums()} fallback={<p class='text-sm text-zinc-400'>Loading album lists...</p>}>
          <For each={albumSections()}>
            {(section) => (
              <div class='flex flex-col gap-1.5 w-full overflow-x-hidden'>
                <Heading1>{section.heading}</Heading1>
                <AlbumHorizontalList albums={section.albums} emptyMessage={`No albums returned for ${section.heading}.`} />
              </div>
            )}
          </For>
        </Show>
      </div>
    </Show>
  );
}

export default Home;
