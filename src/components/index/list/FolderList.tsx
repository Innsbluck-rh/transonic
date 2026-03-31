import { Component, createSignal, onMount } from 'solid-js';
import BrowseList, { BrowseListItem } from '~/components/list/BrowseList';
import { fetchFolderStructureRoots } from '~/features/browse/folderStructureService';
import { fetchMusicFolders } from '~/features/browse/service';

interface FolderListProps {}

const FolderList: Component<FolderListProps> = (props) => {
  const [items, setItems] = createSignal<BrowseListItem[]>([]);

  onMount(async () => {
    try {
      const res = await fetchMusicFolders();
      // temporaly use first
      const library = res.musicFolders[0];
      const structureRoots = await fetchFolderStructureRoots({
        libraryId: library.id,
      });

      setItems(
        structureRoots.rootNodes.map((node) => {
          return {
            id: node.id,
            name: node.name,
            href: `/browse/folders/${encodeURIComponent(library.id)}/${encodeURIComponent(node.id)}`,
          } as BrowseListItem;
        })
      );
    } catch (e) {
      console.error(e);
    }
  });

  return <BrowseList items={items()} emptyMessage='Nothing Found in Library' />;
};

export default FolderList;
