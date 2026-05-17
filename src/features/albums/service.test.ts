import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
  getCoverArt: vi.fn(),
  getCoverArtCached: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: mocks.convertFileSrc,
}));

vi.mock('~/bindings', () => ({
  commands: {
    getCoverArt: mocks.getCoverArt,
    getCoverArtCached: mocks.getCoverArtCached,
  },
}));

async function loadService() {
  return import('./service');
}

describe('album cover art service', () => {
  beforeEach(() => {
    vi.resetModules();
    mocks.getCoverArt.mockReset();
    mocks.getCoverArtCached.mockReset();
    mocks.getCoverArtCached.mockResolvedValue({ status: 'ok', data: null });
    mocks.convertFileSrc.mockReset();
    mocks.convertFileSrc.mockImplementation((path: string) => `asset://${path}`);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('deduplicates parallel requests for the same cover art asset', async () => {
    mocks.getCoverArt.mockResolvedValue({
      status: 'ok',
      data: { localPath: 'cache/cover-48.png' },
    });
    const { fetchCoverArtAssetUrl } = await loadService();
    const payload = { profileId: 'profile-a', coverArtId: 'cover-1', size: 48 };

    const [first, second] = await Promise.all([fetchCoverArtAssetUrl(payload), fetchCoverArtAssetUrl(payload)]);

    expect(first).toBe('asset://cache/cover-48.png');
    expect(second).toBe('asset://cache/cover-48.png');
    expect(mocks.getCoverArtCached).toHaveBeenCalledTimes(1);
    expect(mocks.getCoverArt).toHaveBeenCalledTimes(1);
  });

  it('returns a cache-only hit without fetching cover art', async () => {
    mocks.getCoverArtCached.mockResolvedValue({
      status: 'ok',
      data: { localPath: 'cache/cover-512.png' },
    });
    const { fetchCoverArtAssetUrl } = await loadService();

    const result = await fetchCoverArtAssetUrl({
      profileId: 'profile-a',
      coverArtId: 'cover-1',
      size: 48,
      cachedFallbackSizes: [512],
    });

    expect(result).toBe('asset://cache/cover-512.png');
    expect(mocks.getCoverArtCached).toHaveBeenCalledWith({
      profileId: 'profile-a',
      coverArtId: 'cover-1',
      size: 48,
      cachedFallbackSizes: [512],
    });
    expect(mocks.getCoverArt).not.toHaveBeenCalled();
  });

  it('keeps different sizes as separate cache entries', async () => {
    mocks.getCoverArt
      .mockResolvedValueOnce({
        status: 'ok',
        data: { localPath: 'cache/cover-48.png' },
      })
      .mockResolvedValueOnce({
        status: 'ok',
        data: { localPath: 'cache/cover-224.png' },
      });
    const { fetchCoverArtAssetUrl } = await loadService();

    const [small, large] = await Promise.all([
      fetchCoverArtAssetUrl({ profileId: 'profile-a', coverArtId: 'cover-1', size: 48 }),
      fetchCoverArtAssetUrl({ profileId: 'profile-a', coverArtId: 'cover-1', size: 224 }),
    ]);

    expect(small).toBe('asset://cache/cover-48.png');
    expect(large).toBe('asset://cache/cover-224.png');
    expect(mocks.getCoverArt).toHaveBeenCalledTimes(2);
  });

  it('keeps fallback size policy as part of the cache key', async () => {
    mocks.getCoverArt
      .mockResolvedValueOnce({
        status: 'ok',
        data: { localPath: 'cache/cover-48.png' },
      })
      .mockResolvedValueOnce({
        status: 'ok',
        data: { localPath: 'cache/cover-48-with-fallback.png' },
      });
    const { fetchCoverArtAssetUrl } = await loadService();

    const [withoutFallback, withFallback] = await Promise.all([
      fetchCoverArtAssetUrl({ profileId: 'profile-a', coverArtId: 'cover-1', size: 48 }),
      fetchCoverArtAssetUrl({
        profileId: 'profile-a',
        coverArtId: 'cover-1',
        size: 48,
        cachedFallbackSizes: [512],
      }),
    ]);

    expect(withoutFallback).toBe('asset://cache/cover-48.png');
    expect(withFallback).toBe('asset://cache/cover-48-with-fallback.png');
    expect(mocks.getCoverArt).toHaveBeenCalledTimes(2);
  });

  it('deduplicates failed requests and caches the null result', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    mocks.getCoverArt.mockResolvedValue({
      status: 'error',
      error: 'missing cover art',
    });
    const { fetchCoverArtAssetUrl } = await loadService();
    const payload = { profileId: 'profile-a', coverArtId: 'cover-1', size: 48 };

    const [first, second] = await Promise.all([fetchCoverArtAssetUrl(payload), fetchCoverArtAssetUrl(payload)]);
    const third = await fetchCoverArtAssetUrl(payload);

    expect(first).toBeNull();
    expect(second).toBeNull();
    expect(third).toBeNull();
    expect(mocks.getCoverArtCached).toHaveBeenCalledTimes(1);
    expect(mocks.getCoverArt).toHaveBeenCalledTimes(1);
    expect(consoleErrorSpy).toHaveBeenCalledTimes(1);
  });
});
