// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActiveSession } from '~/bindings';
import BrowseList from '~/components/common/list/browse/BrowseList';
import type { BrowseListItem } from '~/features/browse/service';
import { setSessionStore } from '~/stores/SessionStore';

const mocks = vi.hoisted(() => ({
  fetchCoverArtAssetUrl: vi.fn(),
}));

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

vi.mock('@solidjs/router', () => ({
  useLocation: () => ({ pathname: '/' }),
}));

vi.mock('~/features/navigation/useSPNavigate', () => ({
  useSPNavigate: () => vi.fn(),
}));

vi.mock('~/features/albums/service', () => ({
  fetchCoverArtAssetUrl: mocks.fetchCoverArtAssetUrl,
}));

function activeSession(profileId: string): ActiveSession {
  return {
    profileId,
    normalizedServerUrl: 'https://music.example',
    username: 'user',
    authKind: 'password',
    apiVersion: '1.16.1',
    serverType: null,
    serverVersion: null,
    capabilityMatrix: {
      openSubsonic: true,
      rawExtensions: [],
      apiKeyAuth: false,
      indexBasedQueue: false,
      playbackReport: false,
      transcoding: false,
      transcodeOffset: false,
      songLyrics: false,
    },
  };
}

function flushPromises() {
  return new Promise<void>((resolve) => setTimeout(resolve, 0));
}

async function waitFor(assertion: () => void) {
  let lastError: unknown;
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      assertion();
      return;
    } catch (error) {
      lastError = error;
      await flushPromises();
    }
  }

  throw lastError;
}

type IntersectionCallback = (entries: { isIntersecting: boolean; target: Element }[]) => void;

class FakeIntersectionObserver {
  static instances: FakeIntersectionObserver[] = [];
  readonly observed: Element[] = [];
  private readonly callback: IntersectionCallback;

  constructor(callback: IntersectionCallback) {
    this.callback = callback;
    FakeIntersectionObserver.instances.push(this);
  }

  observe(element: Element) {
    this.observed.push(element);
  }

  unobserve() {}

  disconnect() {}

  enter(element: Element) {
    this.callback([{ isIntersecting: true, target: element }]);
  }
}

describe('BrowseList', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    setSessionStore('activeSession', activeSession('profile-a'));
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    setSessionStore('activeSession', null);
    mocks.fetchCoverArtAssetUrl.mockReset();
    vi.unstubAllGlobals();
    FakeIntersectionObserver.instances = [];
  });

  it('requests cover art only for artist rows that enter the viewport', async () => {
    vi.stubGlobal('IntersectionObserver', FakeIntersectionObserver);
    mocks.fetchCoverArtAssetUrl.mockImplementation((payload: { coverArtId: string }) => Promise.resolve(`asset://${payload.coverArtId}`));
    const items: BrowseListItem[] = [
      { type: 'artist', id: 'a1', name: 'Artist 1', coverArtId: 'ar-a1' },
      { type: 'artist', id: 'a2', name: 'Artist 2', coverArtId: 'ar-a2' },
      { type: 'artist', id: 'a3', name: 'Artist 3', coverArtId: null },
      { type: 'folder', id: 'n1', name: 'Folder 1' },
    ];

    dispose = render(() => <BrowseList items={items} emptyMessage='' />, container);

    const observer = FakeIntersectionObserver.instances[0];
    expect(observer.observed.map((element) => (element as HTMLElement).dataset.coverArtId)).toEqual(['ar-a1', 'ar-a2']);
    expect(mocks.fetchCoverArtAssetUrl).not.toHaveBeenCalled();

    observer.enter(observer.observed[0]);

    await waitFor(() => {
      expect(rowOf('Artist 1').querySelector('img')?.getAttribute('src')).toBe('asset://ar-a1');
    });
    expect(mocks.fetchCoverArtAssetUrl).toHaveBeenCalledTimes(1);
    expect(mocks.fetchCoverArtAssetUrl).toHaveBeenCalledWith(expect.objectContaining({ coverArtId: 'ar-a1' }));
    expect(rowOf('Artist 2').querySelector('img')).toBeNull();
  });

  it('keeps the placeholder when the cover art cannot be resolved', async () => {
    mocks.fetchCoverArtAssetUrl.mockResolvedValue(null);
    const items: BrowseListItem[] = [{ type: 'artist', id: 'a1', name: 'Artist 1', coverArtId: 'ar-a1' }];

    dispose = render(() => <BrowseList items={items} emptyMessage='' />, container);

    await waitFor(() => {
      expect(mocks.fetchCoverArtAssetUrl).toHaveBeenCalledTimes(1);
    });
    await flushPromises();
    expect(rowOf('Artist 1').querySelector('img')).toBeNull();
  });

  function rowOf(label: string): HTMLElement {
    const labelElement = Array.from(container.querySelectorAll('p')).find((element) => element.textContent === label);
    if (!labelElement?.parentElement) {
      throw new Error(`Row not found: ${label}`);
    }
    return labelElement.parentElement;
  }
});
