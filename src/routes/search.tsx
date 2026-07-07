import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, createSignal, onMount, Show } from 'solid-js';
import { commands, type SearchResponse } from '~/bindings';
import AlbumList from '~/components/common/list/album/AlbumList';
import BrowseList, { type BrowseListItem } from '~/components/common/list/index/BrowseList';
import SearchSongList from '~/components/common/list/search/SearchSongList';
import LoadCircle from '~/components/common/LoadCircle';
import Heading3 from '~/components/common/text/Heading3';
import RouteHeader from '~/components/common/text/RouteHeader';
import { resolveAlbumRoute, resolveArtistRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';
import { isSP } from '~/utils/isSP';

const Search: Component = () => {
  const navigate = useSPNavigate();
  const [results, setResults] = createSignal<SearchResponse | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let searchInput!: HTMLInputElement;

  onMount(() => {
    if (!isSP()) searchInput.focus();
  });

  const artistItems = createMemo<BrowseListItem[]>(() =>
    (results()?.artists ?? []).map((artist) => ({
      id: artist.id,
      name: artist.name,
      href: resolveArtistRoute(artist.id),
    }))
  );

  async function searchQuery(query: string) {
    setLoading(true);
    setError(null);

    try {
      if (!query.trim()) {
        setLoading(false);
        setResults(null);
        return;
      }
      const result = await commands.search({ query });
      if (result.status === 'error') {
        setResults(null);
        setError(result.error);
        return;
      }

      setResults(result.data);
    } catch (invokeError) {
      console.error(invokeError);
      setResults(null);
      setError('Failed to search.');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div
      class='home-surface-root relative p-0'
      style={{
        'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
        'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
      }}
    >
      <RouteHeader title='Search' />

      <div class='flex w-full justify-center gap-3 p-2'>
        <input
          ref={(ref) => (searchInput = ref)}
          class='flex-1'
          type='search'
          placeholder='Search Artists/Albums/Songs...'
          onInput={(event) => {
            void searchQuery(event.currentTarget.value);
          }}
        />
        <div class='flex w-7 items-center justify-center'>
          <Show when={loading()} fallback={<Icon icon='material-symbols:search' class='text-xl' />}>
            <LoadCircle />
          </Show>
        </div>
      </div>

      <Show when={error()}>{(message) => <p class='px-3 py-2 text-sm text-red-500'>{message()}</p>}</Show>

      <div class='flex flex-col gap-2'>
        <Show when={artistItems().length > 0}>
          <section class='flex flex-col'>
            <div class='bg-primary-bg border-secondary-border border-b p-2'>
              <Heading3>artists</Heading3>
            </div>
            <BrowseList items={artistItems()} emptyMessage='' />
          </section>
        </Show>

        <Show when={(results()?.albums.length ?? 0) > 0}>
          <section class='flex flex-col'>
            <div class='bg-primary-bg border-secondary-border border-b p-2'>
              <Heading3>albums</Heading3>
            </div>
            <AlbumList
              albums={results()?.albums ?? []}
              emptyMessage=''
              onItemClick={(album) => {
                navigate(resolveAlbumRoute(album.id));
              }}
            />
          </section>
        </Show>

        <Show when={(results()?.songs.length ?? 0) > 0}>
          <section class='flex flex-col'>
            <div class='bg-primary-bg border-secondary-border border-b p-2'>
              <Heading3>songs</Heading3>
            </div>
            <SearchSongList songs={results()?.songs ?? []} />
          </section>
        </Show>
      </div>
    </div>
  );
};

export default Search;
