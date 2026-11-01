import { createSignal, For, Match, onMount, Show, Switch } from 'solid-js';
import { commands, type AlbumListContext, type AlbumListItem } from '~/bindings';
import AlbumGrid from '~/components/common/list/album/AlbumGrid';
import HorizontalAlbumItem from '~/components/common/list/album/item/HorizontalAlbumItem';
import LoadCircle from '~/components/common/LoadCircle';
import Heading2 from '~/components/common/text/Heading2';
import RouteHeader from '~/components/common/text/RouteHeader';
import { resolveAlbumRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

type HomeAlbumSection = {
  heading: string;
  context: AlbumListContext;
  albums: AlbumListItem[];
  size: number;
};

const HOME_ALBUM_CONTEXTS: Array<Pick<HomeAlbumSection, 'heading' | 'context' | 'size'>> = [
  { heading: 'recently played', context: 'recent', size: 4 },
  { heading: 'recently added', context: 'newest', size: 6 },
  { heading: 'random picks', context: 'random', size: 6 },
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
            size: section.size,
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
    <div
      class='home-surface-root overflow-y-auto p-0'
      style={{
        'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
        'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
      }}
    >
      <RouteHeader title='Home' />
      <div class='mt-2' />

      <Show when={albumError()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

      <div class='flex flex-col gap-4'>
        <Show
          when={!isLoadingAlbums()}
          fallback={
            <div class='flex size-full items-center justify-center'>
              <LoadCircle />
            </div>
          }
        >
          <For each={albumSections()}>
            {(section) => (
              <div class='flex flex-col'>
                <Heading2 class='px-2'>{section.heading}</Heading2>
                <Switch
                  fallback={
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
                  }
                >
                  <Match when={section.context === 'recent'}>
                    <Show
                      when={section.albums.length > 0}
                      fallback={<p class='archivo text-secondary-text mt-4 text-center italic'>No recent albums.</p>}
                    >
                      <div class='grid w-full grid-cols-2 gap-1.5 px-3 py-2 xl:grid-cols-3'>
                        <For each={section.albums}>
                          {(album, _i) => {
                            return <HorizontalAlbumItem album={album} />;
                          }}
                        </For>
                      </div>
                    </Show>
                  </Match>
                </Switch>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  );
}

export default SPHome;
