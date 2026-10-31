import { commands } from '~/bindings';
import { type BrowseMode, resolveArtistRoute, resolveFolderRoute } from '~/features/navigation/routes';

type BrowseListItemBase = {
  id: string;
  name: string;
  href?: string;
};

export type BrowseListItem = (BrowseListItemBase & { type: 'artist'; coverArtId: string | null }) | (BrowseListItemBase & { type: 'folder' });

export async function loadArtistBrowseItems(): Promise<BrowseListItem[]> {
  const mfResult = await commands.getMusicFolders();
  if (mfResult.status === 'error') {
    console.error(mfResult.error);
    return [];
  }
  const library = mfResult.data.musicFolders[0];
  if (!library) return [];

  const artistsResult = await commands.getArtists({ musicFolderId: library.id });
  if (artistsResult.status === 'error') {
    console.error(artistsResult.error);
    return [];
  }

  return artistsResult.data.indexes
    .flatMap((index) => index.artists)
    .map((artist) => ({
      type: 'artist',
      id: artist.id,
      name: artist.name,
      href: resolveArtistRoute(artist.id),
      coverArtId: artist.coverArtId,
    }));
}

export async function loadFolderBrowseItems(): Promise<BrowseListItem[]> {
  const mfResult = await commands.getMusicFolders();
  if (mfResult.status === 'error') {
    console.error(mfResult.error);
    return [];
  }
  // temporaly use first
  const library = mfResult.data.musicFolders[0];
  if (!library) return [];

  const fsrResult = await commands.getFolderStructureRoots({ libraryId: library.id });
  if (fsrResult.status === 'error') {
    console.error(fsrResult.error);
    return [];
  }

  return fsrResult.data.rootNodes.map((node) => ({
    type: 'folder',
    id: node.id,
    name: node.name,
    href: resolveFolderRoute(library.id, node.id),
  }));
}

export function loadBrowseItems(mode: BrowseMode): Promise<BrowseListItem[]> {
  switch (mode) {
    case 'Artists':
      return loadArtistBrowseItems();
    case 'Folder Structures':
      return loadFolderBrowseItems();
  }
}
