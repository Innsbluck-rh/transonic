import { isSP } from '~/utils/isSP';

export const BROWSE_MODES = ['Folder Structures', 'Artists'] as const;

export type BrowseMode = (typeof BROWSE_MODES)[number];
export type RouteTarget = 'pc' | 'sp';

function resolveRouteTarget(target?: RouteTarget) {
  return target ?? (isSP() ? 'sp' : 'pc');
}

function encodePathSegment(value: string) {
  return encodeURIComponent(value);
}

export function resolveHomeRoute(target?: RouteTarget) {
  return resolveRouteTarget(target) === 'sp' ? '/sp' : '/home';
}

export function resolveSearchRoute(target?: RouteTarget) {
  return resolveRouteTarget(target) === 'sp' ? '/sp/search' : '/search';
}

export function resolveSettingsRoute(target?: RouteTarget) {
  return resolveRouteTarget(target) === 'sp' ? '/sp/settings' : '/settings';
}

export function resolveArtistRoute(artistId: string, target?: RouteTarget) {
  const encodedArtistId = encodePathSegment(artistId);
  return resolveRouteTarget(target) === 'sp' ? `/sp/browse/artists/${encodedArtistId}` : `/browse/artists/${encodedArtistId}`;
}

export function resolveFolderRoute(libraryId: string, nodeId: string | null | undefined, target?: RouteTarget) {
  const routeTarget = resolveRouteTarget(target);
  const encodedLibraryId = encodePathSegment(libraryId);
  if (!nodeId) {
    return routeTarget === 'sp' ? `/sp/browse/folders/${encodedLibraryId}` : `/browse/folders/${encodedLibraryId}`;
  }

  const encodedNodeId = encodePathSegment(nodeId);
  return routeTarget === 'sp' ? `/sp/browse/folders/${encodedLibraryId}/${encodedNodeId}` : `/browse/folders/${encodedLibraryId}/${encodedNodeId}`;
}

export function resolveAlbumRoute(albumId: string, target?: RouteTarget) {
  const encodedAlbumId = encodePathSegment(albumId);
  return resolveRouteTarget(target) === 'sp' ? `/sp/browse/albums/${encodedAlbumId}` : `/browse/album/${encodedAlbumId}`;
}

export function resolveSPBrowseRoute() {
  return '/sp/browse';
}
