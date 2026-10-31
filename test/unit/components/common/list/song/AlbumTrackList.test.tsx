import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import AlbumTrackList from '~/components/common/list/song/AlbumTrackList';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { resetConnectStore, setSharedPlayback, sharedPlayback } from '~test/fixtures/connect';
import { playbackStatus, resetPlaybackStore, song } from '~test/fixtures/playback';

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

vi.mock('~/features/menu', () => ({
  buildSongMenuItems: () => [],
}));

vi.mock('~/features/menu/useContextMenu', () => ({
  default: () => ({}),
}));

vi.mock('~/bindings', () => ({
  commands: {
    playbackSetQueue: vi.fn(),
  },
}));

describe('AlbumTrackList', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    resetPlaybackStore();
    resetConnectStore();
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    vi.clearAllMocks();
  });

  it('marks the shared current song when Connect playback is active', () => {
    const staleLocalSong = song('local-current');
    const remoteSongs = [song('remote-1'), song('remote-2')];
    setPlaybackStore('status', playbackStatus({ queue: [staleLocalSong], currentIndex: 0 }));
    setSharedPlayback(sharedPlayback({ queue: remoteSongs, currentIndex: 1 }));

    dispose = render(() => <AlbumTrackList songs={[staleLocalSong, ...remoteSongs]} />, container);

    const rows = container.querySelectorAll('.song-item');
    expect(rows[0].getAttribute('data-current')).toBeNull();
    expect(rows[1].getAttribute('data-current')).toBeNull();
    expect(rows[2].getAttribute('data-current')).toBe('true');
  });
});
