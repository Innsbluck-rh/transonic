import { createSignal, For, onMount, Show } from 'solid-js';
import { commands, type AlbumListContext, type AlbumListItem } from '~/bindings';
import Heading2 from '~/components/common/Heading2';
import AlbumGrid from '~/components/common/list/album/AlbumGrid';
import RouteHeader from '~/components/common/RouteHeader';
import { resolveAlbumRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';

type HomeAlbumSection = {
  heading: string;
  context: AlbumListContext;
  albums: AlbumListItem[];
};

const HOME_ALBUM_CONTEXTS: Array<Pick<HomeAlbumSection, 'heading' | 'context'>> = [
  { heading: 'recently played', context: 'recent' },
  { heading: 'recently added', context: 'newest' },
  { heading: 'random picks', context: 'random' },
];

function SPHome() {
  const navigate = useSPNavigate();
  const [albumSections, setAlbumSections] = createSignal<HomeAlbumSection[]>([]);
  const [isLoadingAlbums, setIsLoadingAlbums] = createSignal(false);
  const [albumError, setAlbumError] = createSignal<string | null>(null);

  async function loadHomeAlbumSections() {
    setIsLoadingAlbums(true);
    setAlbumError(null);

    try {
      const sectionResults = await Promise.all(
        HOME_ALBUM_CONTEXTS.map(async (section) => {
          const result = await commands.getAlbumList({
            context: section.context,
            size: 3,
            offset: null,
            fromYear: null,
            toYear: null,
            genre: null,
            musicFolderId: null,
          });

          return { section, result };
        })
      );

      const failed = sectionResults.find(({ result }) => result.status === 'error');
      if (failed && failed.result.status === 'error') {
        setAlbumError(failed.result.error);
        setAlbumSections([]);
        return;
      }

      const sections = sectionResults.map(({ section, result }) => {
        if (result.status === 'error') {
          return {
            ...section,
            albums: [],
          };
        }

        return {
          ...section,
          albums: result.data.albums,
        };
      });

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
    <div class='home-surface-root overflow-y-auto p-0'>
      <RouteHeader title='Home' />
      <div class='mt-2' />

      <Show when={albumError()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <div class='flex flex-col gap-4'>
        <Show when={!isLoadingAlbums()} fallback={<p class='text-sm text-zinc-400'>Loading album lists...</p>}>
          <For each={albumSections()}>
            {(section) => (
              <div class='flex flex-col'>
                <Heading2 class='px-2'>{section.heading}</Heading2>
                <div class='m-3'>
                  <AlbumGrid
                    inline
                    albums={section.albums}
                    emptyMessage={`No albums returned for ${section.heading}.`}
                    onItemClick={(album) => {
                      navigate(resolveAlbumRoute(album.id));
                    }}
                  />
                </div>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  );
}

export default SPHome;
