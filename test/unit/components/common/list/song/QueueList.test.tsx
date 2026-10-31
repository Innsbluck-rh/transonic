import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import QueueList from '~/components/common/list/song/QueueList';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { resetConnectStore, setSharedPlayback, sharedPlayback } from '~test/fixtures/connect';
import { playbackStatus, resetPlaybackStore, song } from '~test/fixtures/playback';

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

vi.mock('@atlaskit/pragmatic-drag-and-drop/combine', () => ({
  combine: () => () => {},
}));

vi.mock('@atlaskit/pragmatic-drag-and-drop/element/adapter', () => ({
  draggable: () => () => {},
  dropTargetForElements: () => () => {},
}));

vi.mock('~/features/menu', () => ({
  buildQueueItemMenuItems: () => [],
}));

vi.mock('~/features/menu/useContextMenu', () => ({
  default: () => ({}),
}));

vi.mock('~/features/navigation/useSPNavigate', () => ({
  useSPNavigate: () => vi.fn(),
}));

vi.mock('~/bindings', () => ({
  commands: {
    playbackMoveQueueIndex: vi.fn().mockResolvedValue({ status: 'ok' }),
    playbackPlayQueueIndex: vi.fn(),
  },
}));

describe('QueueList', () => {
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

  it('highlights the shared current queue row when the local queue index is stale', () => {
    const remoteQueue = [song('remote-1'), song('remote-2')];
    setPlaybackStore('status', playbackStatus({ queue: [remoteQueue[1]], currentIndex: 0 }));
    setSharedPlayback(sharedPlayback({ queue: remoteQueue, currentIndex: 1 }));

    dispose = render(() => <QueueList queue={remoteQueue} />, container);

    const rows = container.querySelectorAll('.song-item');
    expect(rows[0].getAttribute('data-current')).toBeNull();
    expect(rows[1].getAttribute('data-current')).toBe('true');
  });
});
