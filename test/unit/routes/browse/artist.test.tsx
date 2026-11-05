import { createStore } from 'solid-js/store';
import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ArtistInfo2Response, ArtistResponse } from '~/bindings';
import BrowseArtist from '~/routes/browse/artist';
import { setSessionStore } from '~/stores/SessionStore';
import { activeSession } from '~test/fixtures/session';
import { waitFor } from '~test/helpers/async';

const mocks = vi.hoisted(() => ({
  getArtist: vi.fn(),
  getArtistInfo2: vi.fn(),
  fetchArtistImageAssetUrl: vi.fn(),
  fetchCoverArtAssetUrl: vi.fn(),
  params: { id: '' } as { id: string },
}));

vi.mock('@tauri-apps/plugin-os', () => ({
  platform: () => 'windows',
}));

vi.mock('@solidjs/router', () => ({
  useParams: () => mocks.params,
}));

vi.mock('~/features/navigation/useSPNavigate', () => ({
  useSPNavigate: () => vi.fn(),
}));

vi.mock('~/bindings', () => ({
  commands: {
    getArtist: mocks.getArtist,
    getArtistInfo2: mocks.getArtistInfo2,
    settingsUpdate: vi.fn(),
  },
}));

vi.mock('~/features/artist/service', () => ({
  fetchArtistImageAssetUrl: mocks.fetchArtistImageAssetUrl,
}));

vi.mock('~/features/albums/service', () => ({
  fetchCoverArtAssetUrl: mocks.fetchCoverArtAssetUrl,
}));

function artist(id: string, overrides: Partial<ArtistResponse> = {}): ArtistResponse {
  return {
    id,
    name: `Artist ${id}`,
    coverArtId: null,
    albumCount: 0,
    artistImageUrl: null,
    starred: null,
    musicBrainzId: null,
    sortName: null,
    roles: [],
    albums: [],
    ...overrides,
  };
}

function artistInfo(overrides: Partial<ArtistInfo2Response> = {}): ArtistInfo2Response {
  return {
    biography: null,
    musicBrainzId: null,
    lastFmUrl: null,
    smallImageUrl: null,
    mediumImageUrl: null,
    largeImageUrl: null,
    similarArtists: [],
    ...overrides,
  };
}

describe('BrowseArtist', () => {
  let container: HTMLDivElement;
  let dispose: (() => void) | undefined;

  beforeEach(() => {
    setSessionStore('activeSession', activeSession('profile-a'));
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    dispose = undefined;
    container.remove();
    setSessionStore('activeSession', null);
    mocks.getArtist.mockReset();
    mocks.getArtistInfo2.mockReset();
    mocks.fetchArtistImageAssetUrl.mockReset();
    mocks.fetchCoverArtAssetUrl.mockReset();
  });

  it('clears the artist image when navigating to an artist without one', async () => {
    const artists: Record<string, ArtistResponse> = {
      a: artist('a'),
      b: artist('b'),
    };
    const infos: Record<string, ArtistInfo2Response> = {
      a: artistInfo({ largeImageUrl: 'https://images.example/a.jpg' }),
      b: artistInfo(),
    };
    mocks.getArtist.mockImplementation(({ id }: { id: string }) => Promise.resolve({ status: 'ok', data: artists[id] }));
    mocks.getArtistInfo2.mockImplementation(({ id }: { id: string }) => Promise.resolve({ status: 'ok', data: infos[id] }));
    mocks.fetchArtistImageAssetUrl.mockImplementation((url: string) => Promise.resolve(`asset://${url}`));

    const [params, setParams] = createStore({ id: 'a' });
    mocks.params = params;

    dispose = render(() => <BrowseArtist />, container);

    await waitFor(() => expect(container.querySelector('img')?.getAttribute('src')).toBe('asset://https://images.example/a.jpg'));

    setParams('id', 'b');

    await waitFor(() => expect(container.textContent).toContain('Artist b'));
    expect(container.querySelector('img')).toBeNull();
  });
});
