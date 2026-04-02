import { Component, createSignal, onMount, Show } from 'solid-js';
import { commands } from '~/bindings';
import LoadCircle from '~/components/common/LoadCircle';
import BrowseList, { BrowseListItem } from '~/components/common/list/index/BrowseList';

interface ArtistListProps {}

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
    // temporaly use first
    const library = mfData.musicFolders[0];
    const aiResult = await commands.getArtistIndexes({ musicFolderId: library.id });
    if (aiResult.status === 'error') {
      console.error(aiResult.error);
      setLoading(false);
      return;
    }

    setItems(
      aiResult.data.artists.map((artist) => {
        return {
          id: artist.id,
          name: artist.name,
          href: `/browse/artists/${encodeURIComponent(artist.id)}`,
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
