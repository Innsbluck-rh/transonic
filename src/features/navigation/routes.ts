import { isSP } from '~/utils/isSP';

export const HOME_ROUTE = '/home';
export const MOBILE_HOME_ROUTE = '/sp/index/artists';
export const INIT_LOGIN_ROUTE = '/init_login';
export const HOME_ERROR_ROUTE = '/home/error';

export function resolveHomeRoute() {
  return isSP() ? MOBILE_HOME_ROUTE : HOME_ROUTE;
}

export function resolveArtistRoute(artistId: string) {
  const encodedArtistId = encodeURIComponent(artistId);
  return isSP() ? `/sp/artists/${encodedArtistId}` : `/browse/artists/${encodedArtistId}`;
}

export function resolveFolderRoute(libraryId: string, nodeId: string | null | undefined) {
  const encodedLibraryId = encodeURIComponent(libraryId);
  if (!nodeId) {
    return isSP() ? `/sp/folders/${encodedLibraryId}` : `/browse/folders/${encodedLibraryId}`;
  }

  const encodedNodeId = encodeURIComponent(nodeId);
  return isSP() ? `/sp/folders/${encodedLibraryId}/${encodedNodeId}` : `/browse/folders/${encodedLibraryId}/${encodedNodeId}`;
}

export function resolveAlbumRoute(albumId: string) {
  const encodedAlbumId = encodeURIComponent(albumId);
  return isSP() ? `/sp/albums/${encodedAlbumId}` : `/browse/album/${encodedAlbumId}`;
}

export function resolveBrowseIndexRoute(mode: 'Artists' | 'Folder Structures') {
  if (isSP()) {
    return mode === 'Artists' ? '/sp/index/artists' : '/sp/index/folders';
  }

  return mode === 'Artists' ? '/browse/artists' : '/browse/folders';
}
