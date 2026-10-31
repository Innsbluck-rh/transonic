import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { loadArtistBrowseItems, loadFolderBrowseItems } from '~/features/browse/service';

const mocks = vi.hoisted(() => ({
  getMusicFolders: vi.fn(),
  getArtists: vi.fn(),
  getFolderStructureRoots: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-os', () => ({
  platform: () => 'windows',
}));

vi.mock('~/bindings', () => ({
  commands: {
    getMusicFolders: mocks.getMusicFolders,
    getArtists: mocks.getArtists,
    getFolderStructureRoots: mocks.getFolderStructureRoots,
  },
}));

function artist(id: string, coverArtId: string | null) {
  return {
    id,
    name: `Artist ${id}`,
    coverArtId,
    albumCount: null,
    artistImageUrl: null,
    starred: null,
    musicBrainzId: null,
    sortName: null,
    roles: [],
  };
}

describe('browse item loaders', () => {
  beforeEach(() => {
    mocks.getMusicFolders.mockResolvedValue({
      status: 'ok',
      data: { musicFolders: [{ id: 'lib-1', name: 'Music' }] },
    });
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.resetAllMocks();
    vi.restoreAllMocks();
  });

  it('maps artists from every index group into artist items', async () => {
    mocks.getArtists.mockResolvedValue({
      status: 'ok',
      data: {
        ignoredArticles: null,
        indexes: [
          { name: 'A', artists: [artist('a1', 'ar-a1')] },
          { name: 'B', artists: [artist('b1', null)] },
        ],
      },
    });

    const items = await loadArtistBrowseItems();

    expect(mocks.getArtists).toHaveBeenCalledWith({ musicFolderId: 'lib-1' });
    expect(items).toEqual([
      { type: 'artist', id: 'a1', name: 'Artist a1', href: '/browse/artists/a1', coverArtId: 'ar-a1' },
      { type: 'artist', id: 'b1', name: 'Artist b1', href: '/browse/artists/b1', coverArtId: null },
    ]);
  });

  it('maps folder structure roots into folder items', async () => {
    mocks.getFolderStructureRoots.mockResolvedValue({
      status: 'ok',
      data: {
        libraryId: 'lib-1',
        libraryName: 'Music',
        source: 'directory',
        rootNodes: [{ id: 'n1', name: 'Rock' }],
      },
    });

    const items = await loadFolderBrowseItems();

    expect(mocks.getFolderStructureRoots).toHaveBeenCalledWith({ libraryId: 'lib-1' });
    expect(items).toEqual([{ type: 'folder', id: 'n1', name: 'Rock', href: '/browse/folders/lib-1/n1' }]);
  });

  it('returns no items when the server has no music folder', async () => {
    mocks.getMusicFolders.mockResolvedValue({ status: 'ok', data: { musicFolders: [] } });

    expect(await loadArtistBrowseItems()).toEqual([]);
    expect(await loadFolderBrowseItems()).toEqual([]);
    expect(mocks.getArtists).not.toHaveBeenCalled();
    expect(mocks.getFolderStructureRoots).not.toHaveBeenCalled();
  });

  it('returns no items when a request fails', async () => {
    mocks.getArtists.mockResolvedValue({ status: 'error', error: 'boom' });
    mocks.getFolderStructureRoots.mockResolvedValue({ status: 'error', error: 'boom' });

    expect(await loadArtistBrowseItems()).toEqual([]);
    expect(await loadFolderBrowseItems()).toEqual([]);
  });
});
