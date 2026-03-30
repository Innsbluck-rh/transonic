import {
  commands,
  type FolderStructureAlbumsRequest,
  type FolderStructureAlbumsResponse,
  type FolderStructureRootsRequest,
  type FolderStructureRootsResponse,
} from '~/bindings';
import { folderStructureNodeCacheKey, folderStructureStore, setFolderStructureStore } from '~/stores/browse/FolderStructureStore';

const pendingRootsRequests = new Map<string, Promise<FolderStructureRootsResponse>>();
const pendingAlbumsRequests = new Map<string, Promise<FolderStructureAlbumsResponse>>();

export function setFolderStructureSelectedLibraryId(libraryId: string | null) {
  setFolderStructureStore('selectedLibraryId', libraryId);
}

export function syncFolderStructureSession(profileId: string | null) {
  if (folderStructureStore.sessionProfileId === profileId) {
    return;
  }

  pendingRootsRequests.clear();
  pendingAlbumsRequests.clear();
  setFolderStructureStore('sessionProfileId', profileId);
  setFolderStructureStore('selectedLibraryId', null);
  setFolderStructureStore('rootsByLibraryId', {});
  setFolderStructureStore('albumsByNodeKey', {});
}

export async function fetchFolderStructureRoots(payload: FolderStructureRootsRequest) {
  const cached = folderStructureStore.rootsByLibraryId[payload.libraryId];
  if (cached) {
    return cached;
  }

  const existingRequest = pendingRootsRequests.get(payload.libraryId);
  if (existingRequest) {
    return existingRequest;
  }

  const requestedProfileId = folderStructureStore.sessionProfileId;
  const pendingRequest = commands
    .getFolderStructureRoots(payload)
    .then((response) => {
      if (response.status === 'error') {
        throw new Error(response.error);
      }

      if (folderStructureStore.sessionProfileId === requestedProfileId) {
        setFolderStructureStore('rootsByLibraryId', payload.libraryId, response.data);
      }
      return response.data;
    })
    .catch((error) => {
      pendingRootsRequests.delete(payload.libraryId);
      throw error;
    })
    .finally(() => {
      pendingRootsRequests.delete(payload.libraryId);
    });

  pendingRootsRequests.set(payload.libraryId, pendingRequest);
  return pendingRequest;
}

export async function fetchFolderStructureAlbums(payload: FolderStructureAlbumsRequest) {
  const cacheKey = folderStructureNodeCacheKey(payload.libraryId, payload.nodeId);
  const cached = folderStructureStore.albumsByNodeKey[cacheKey];
  if (cached) {
    return cached;
  }

  const existingRequest = pendingAlbumsRequests.get(cacheKey);
  if (existingRequest) {
    return existingRequest;
  }

  const requestedProfileId = folderStructureStore.sessionProfileId;
  const pendingRequest = commands
    .getFolderStructureAlbums(payload)
    .then((response) => {
      if (response.status === 'error') {
        throw new Error(response.error);
      }

      if (folderStructureStore.sessionProfileId === requestedProfileId) {
        setFolderStructureStore('albumsByNodeKey', cacheKey, response.data);
      }
      return response.data;
    })
    .catch((error) => {
      pendingAlbumsRequests.delete(cacheKey);
      throw error;
    })
    .finally(() => {
      pendingAlbumsRequests.delete(cacheKey);
    });

  pendingAlbumsRequests.set(cacheKey, pendingRequest);
  return pendingRequest;
}
