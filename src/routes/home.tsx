import { createSignal, For, onMount, Show } from 'solid-js';
import { commands, type AlbumListContext, type AlbumListItem } from '~/bindings';
import Heading1 from '~/components/common/Heading1';
import Heading2 from '~/components/common/Heading2';
import AlbumHorizontalList from '~/components/common/list/album/AlbumHorizontalList';
import { replaceQueueWithAlbumAndPlay } from '~/features/playback/album';
import { sessionStore } from '~/stores/SessionStore';

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
      const sectionResults = await Promise.all(
        HOME_ALBUM_CONTEXTS.map(async (section) => {
          const result = await commands.getAlbumList({
            context: section.context,
            size: 8,
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
    <Show
      when={sessionStore.activeSession}
      fallback={
        <div>
          <p>seems like there's no active profile.</p>
        </div>
      }
    >
      <div class='home-surface-root'>
        <Heading1 class='p-3'>Hello.</Heading1>

        <Show when={albumError()}>{(message) => <p class='text-sm text-red-500'>{message()}</p>}</Show>

        <Show when={!isLoadingAlbums()} fallback={<p class='text-sm text-zinc-400'>Loading album lists...</p>}>
          <For each={albumSections()}>
            {(section) => (
              <div class='flex flex-col gap-1 w-full overflow-x-hidden'>
                <Heading2 class='px-3'>{section.heading}</Heading2>
                <AlbumHorizontalList
                  class='px-4'
                  albums={section.albums}
                  emptyMessage={`No albums returned for ${section.heading}.`}
                  onItemClick={async (album) => {
                    replaceQueueWithAlbumAndPlay(album.id);
                  }}
                />
              </div>
            )}
          </For>
        </Show>
      </div>
    </Show>
  );
}

export default Home;
