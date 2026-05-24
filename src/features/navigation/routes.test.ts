import { describe, expect, it } from 'vitest';

import { resolveAlbumRoute, resolveArtistRoute, resolveFolderRoute, resolveHomeRoute, resolveSettingsRoute, resolveSPBrowseRoute } from './routes';

describe('navigation routes', () => {
  it('resolves canonical home and settings routes for each target', () => {
    expect(resolveHomeRoute('pc')).toBe('/home');
    expect(resolveHomeRoute('sp')).toBe('/sp');
    expect(resolveSettingsRoute('pc')).toBe('/settings');
    expect(resolveSettingsRoute('sp')).toBe('/sp/settings');
  });

  it('resolves the SP browse route without a mode', () => {
    expect(resolveSPBrowseRoute()).toBe('/sp/browse');
  });

  it('encodes artist and album ids for each target', () => {
    const id = 'artist/id 1';
    const encodedId = encodeURIComponent(id);

    expect(resolveArtistRoute(id, 'pc')).toBe(`/browse/artists/${encodedId}`);
    expect(resolveArtistRoute(id, 'sp')).toBe(`/sp/browse/artists/${encodedId}`);
    expect(resolveAlbumRoute(id, 'pc')).toBe(`/browse/album/${encodedId}`);
    expect(resolveAlbumRoute(id, 'sp')).toBe(`/sp/browse/albums/${encodedId}`);
  });

  it('encodes folder library and node ids for each target', () => {
    const libraryId = 'library/id 1';
    const nodeId = 'node/id 2';
    const encodedLibraryId = encodeURIComponent(libraryId);
    const encodedNodeId = encodeURIComponent(nodeId);

    expect(resolveFolderRoute(libraryId, null, 'pc')).toBe(`/browse/folders/${encodedLibraryId}`);
    expect(resolveFolderRoute(libraryId, nodeId, 'pc')).toBe(`/browse/folders/${encodedLibraryId}/${encodedNodeId}`);
    expect(resolveFolderRoute(libraryId, null, 'sp')).toBe(`/sp/browse/folders/${encodedLibraryId}`);
    expect(resolveFolderRoute(libraryId, nodeId, 'sp')).toBe(`/sp/browse/folders/${encodedLibraryId}/${encodedNodeId}`);
  });
});
