import { Component, createSignal, onMount, Show } from 'solid-js';
import { commands } from '~/bindings';
import LoadCircle from '~/components/common/LoadCircle';
import BrowseList, { BrowseListItem } from '~/components/common/list/index/BrowseList';
import { resolveArtistRoute, type RouteVariant } from '~/features/navigation/routes';

interface ArtistListProps {
  routeVariant?: RouteVariant;
}

const ArtistList: Component<ArtistListProps> = (props) => {
  const [loading, setLoading] = createSignal<boolean>(false);
  const [items, setItems] = createSignal<BrowseListItem[]>([]);

  onMount(async () => {
    setLoading(true);
    const mfResult = await commands.getMusicFolders();
    if (mfResult.status === 'error') {
      console.error(mfResult.error);
      setLoading(false);
      return;
    }
    const mfData = mfResult.data;
    const library = mfData.musicFolders[0];
    if (!library) {
      setItems([]);
      setLoading(false);
      return;
    }

    const artistsResult = await commands.getArtists({ musicFolderId: library.id });
    if (artistsResult.status === 'error') {
      console.error(artistsResult.error);
      setLoading(false);
      return;
    }

    const artists = artistsResult.data.indexes.flatMap((index) => index.artists);
    setItems(
      artists.map((artist) => {
        return {
          id: artist.id,
          name: artist.name,
          href: resolveArtistRoute(artist.id, props.routeVariant ?? 'desktop'),
        } as BrowseListItem;
      })
    );
    setLoading(false);
  });

  return (
    <Show
      when={!loading()}
      fallback={
        <div class='my-8 flex justify-self-center'>
          <LoadCircle />
        </div>
      }
    >
      <BrowseList items={items()} emptyMessage='Nothing Found in Library' />
    </Show>
  );
};

export default ArtistList;
