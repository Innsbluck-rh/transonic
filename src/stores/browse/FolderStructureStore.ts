import { createStore } from 'solid-js/store';
import type { FolderStructureAlbumsResponse, FolderStructureRootsResponse } from '~/bindings';

interface FolderStructureState {
  sessionProfileId: string | null;
  selectedLibraryId: string | null;
  rootsByLibraryId: Record<string, FolderStructureRootsResponse>;
  albumsByNodeKey: Record<string, FolderStructureAlbumsResponse>;
}

const [folderStructureStore, setFolderStructureStore] = createStore<FolderStructureState>({
  sessionProfileId: null,
  selectedLibraryId: null,
  rootsByLibraryId: {},
  albumsByNodeKey: {},
});

function folderStructureNodeCacheKey(libraryId: string, nodeId: string) {
  return `${libraryId}::${nodeId}`;
}

export { folderStructureNodeCacheKey, folderStructureStore, setFolderStructureStore };
