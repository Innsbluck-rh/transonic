export type RouteVariant = 'desktop' | 'mobile';

export const HOME_ROUTE = '/home';
export const MOBILE_HOME_ROUTE = '/sp/index/artists';
export const INIT_LOGIN_ROUTE = '/init_login';
export const HOME_ERROR_ROUTE = '/home/error';

export function resolveHomeRoute(routeVariant: RouteVariant) {
  return routeVariant === 'mobile' ? MOBILE_HOME_ROUTE : HOME_ROUTE;
}

export function resolveArtistRoute(artistId: string, routeVariant: RouteVariant) {
  const encodedArtistId = encodeURIComponent(artistId);
  return routeVariant === 'mobile' ? `/sp/artists/${encodedArtistId}` : `/browse/artists/${encodedArtistId}`;
}

export function resolveFolderRoute(libraryId: string, nodeId: string | null | undefined, routeVariant: RouteVariant) {
  const encodedLibraryId = encodeURIComponent(libraryId);
  if (!nodeId) {
    return routeVariant === 'mobile' ? `/sp/folders/${encodedLibraryId}` : `/browse/folders/${encodedLibraryId}`;
  }

  const encodedNodeId = encodeURIComponent(nodeId);
  return routeVariant === 'mobile' ? `/sp/folders/${encodedLibraryId}/${encodedNodeId}` : `/browse/folders/${encodedLibraryId}/${encodedNodeId}`;
}

export function resolveBrowseIndexRoute(mode: 'Artists' | 'Folder Structures', routeVariant: RouteVariant) {
  if (routeVariant === 'mobile') {
    return mode === 'Artists' ? '/sp/index/artists' : '/sp/index/folders';
  }

  return mode === 'Artists' ? '/browse/artists' : '/browse/folders';
}
