import { Component, createSignal, onMount, Show } from 'solid-js';
import { commands } from '~/bindings';
import LoadCircle from '~/components/common/LoadCircle';
import BrowseList, { BrowseListItem } from '~/components/common/list/index/BrowseList';
import { resolveFolderRoute } from '~/features/navigation/routes';

const FolderList: Component = () => {
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
    const fsrResult = await commands.getFolderStructureRoots({
      libraryId: library.id,
    });
    if (fsrResult.status === 'error') {
      console.error(fsrResult.error);
      setLoading(false);
      return;
    }

    setItems(
      fsrResult.data.rootNodes.map((node) => {
        return {
          id: node.id,
          name: node.name,
          href: resolveFolderRoute(library.id, node.id),
        } as BrowseListItem;
      })
    );
    setLoading(false);
  });

  return (
    <Show
      when={!loading()}
      fallback={
        <div class='my-8 flex self-center justify-self-center'>
          <LoadCircle />
        </div>
      }
    >
      <BrowseList items={items()} emptyMessage='Nothing Found in Library' />
    </Show>
  );
};

export default FolderList;
