import { createRoot, createSignal, type Setter } from 'solid-js';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActiveSession } from '~/bindings';
import { setSessionStore } from '~/stores/SessionStore';
import { useCoverArt, type CoverArtState } from './useCoverArt';

const mocks = vi.hoisted(() => ({
  fetchCoverArtAssetUrl: vi.fn(),
}));

vi.mock('./service', () => ({
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((nextResolve) => {
    resolve = nextResolve;
  });
  return { promise, resolve };
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

describe('useCoverArt', () => {
  beforeEach(() => {
    setSessionStore('activeSession', activeSession('profile-a'));
    mocks.fetchCoverArtAssetUrl.mockReset();
  });

  afterEach(() => {
    setSessionStore('activeSession', null);
    vi.restoreAllMocks();
  });

  it('does not expose the previous cover art source while a new id is loading', async () => {
    const nextCoverArt = deferred<string | null>();
    mocks.fetchCoverArtAssetUrl.mockImplementation((payload: { coverArtId: string }) => {
      if (payload.coverArtId === 'cover-a') {
        return Promise.resolve('asset://cover-a');
      }
      if (payload.coverArtId === 'cover-b') {
        return nextCoverArt.promise;
      }
      return Promise.resolve(null);
    });

    let setCoverArtId!: Setter<string>;
    let state!: CoverArtState;
    const dispose = createRoot((dispose) => {
      const [coverArtId, nextSetCoverArtId] = createSignal('cover-a');
      setCoverArtId = nextSetCoverArtId;
      state = useCoverArt(coverArtId, 48);
      return dispose;
    });

    try {
      await waitFor(() => expect(state.src()).toBe('asset://cover-a'));

      setCoverArtId('cover-b');

      expect(state.loading()).toBe(true);
      expect(state.src()).toBeUndefined();

      nextCoverArt.resolve('asset://cover-b');
      await waitFor(() => expect(state.src()).toBe('asset://cover-b'));
    } finally {
      dispose();
    }
  });
});
