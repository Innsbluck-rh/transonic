import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import QueueList from '~/components/common/list/song/QueueList';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { clearQueueSelection, queueSelection, toggleQueueSelection } from '~/stores/QueueSelectionStore';
import { resetConnectStore, setSharedPlayback, sharedPlayback } from '~test/fixtures/connect';
import { playbackStatus, resetPlaybackStore, song } from '~test/fixtures/playback';

const platform = vi.hoisted(() => ({ current: 'windows' }));

vi.mock('@tauri-apps/plugin-os', () => ({
  platform: () => platform.current,
}));

const menuMocks = vi.hoisted(() => ({ showContextMenu: vi.fn() }));

vi.mock('~/features/menu', () => ({
  buildQueueItemMenuItems: () => [],
  buildQueueSelectionMenuItems: () => [],
  showContextMenu: menuMocks.showContextMenu,
}));

vi.mock('~/features/menu/useContextMenu', () => ({
  default: () => ({}),
}));

vi.mock('~/features/navigation/useSPNavigate', () => ({
  useSPNavigate: () => vi.fn(),
}));

const bindingMocks = vi.hoisted(() => ({
  playbackPlayQueueIndex: vi.fn(),
}));

vi.mock('~/bindings', () => ({
  commands: {
    playbackMoveQueueIndices: vi.fn().mockResolvedValue({ status: 'ok' }),
    playbackRemoveQueueIndices: vi.fn().mockResolvedValue({ status: 'ok' }),
    playbackPlayQueueIndex: bindingMocks.playbackPlayQueueIndex,
  },
}));

describe('QueueList', () => {
  let container: HTMLDivElement;
  let dispose: () => void;
  const queue = [song('a'), song('b'), song('c'), song('d')];

  const row = (index: number) => container.querySelectorAll<HTMLElement>('.song-item')[index];
  const handle = (index: number) => container.querySelectorAll<HTMLElement>('[data-queue-handle]')[index];
  const click = (target: HTMLElement, init: MouseEventInit = {}) => target.dispatchEvent(new MouseEvent('click', { bubbles: true, ...init }));

  beforeEach(() => {
    platform.current = 'windows';
    resetPlaybackStore();
    resetConnectStore();
    clearQueueSelection();
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    vi.clearAllMocks();
  });

  const renderQueue = () => {
    setPlaybackStore('status', playbackStatus({ queue, currentIndex: 0 }));
    dispose = render(() => <QueueList queue={queue} />, container);
  };

  it('highlights the shared current queue row when the local queue index is stale', () => {
    const remoteQueue = [song('remote-1'), song('remote-2')];
    setPlaybackStore('status', playbackStatus({ queue: [remoteQueue[1]], currentIndex: 0 }));
    setSharedPlayback(sharedPlayback({ queue: remoteQueue, currentIndex: 1 }));

    dispose = render(() => <QueueList queue={remoteQueue} />, container);

    const rows = container.querySelectorAll('.song-item');
    expect(rows[0].getAttribute('data-current')).toBeNull();
    expect(rows[1].getAttribute('data-current')).toBe('true');
  });

  it('starts a selection with Ctrl+click and, while selecting, turns rows on and off with a plain click', () => {
    renderQueue();

    click(row(1), { ctrlKey: true });
    click(row(3));
    expect(queueSelection()).toEqual({ indices: [1, 3], anchor: 3 });
    expect(container.querySelectorAll('.icon-\\[material-symbols--check-box\\]')).toHaveLength(2);

    click(row(1));
    expect(queueSelection().indices).toEqual([3]);
    expect(bindingMocks.playbackPlayQueueIndex).not.toHaveBeenCalled();
  });

  it('selects anchor..row on Shift+click', () => {
    renderQueue();

    click(row(1), { shiftKey: true });
    click(row(3), { shiftKey: true });

    expect(queueSelection()).toEqual({ indices: [1, 2, 3], anchor: 1 });
  });

  it('plays a row on a plain click, the number column included, when nothing is selected', () => {
    renderQueue();

    click(row(2).querySelector<HTMLElement>('.song-item-leading')!);

    expect(bindingMocks.playbackPlayQueueIndex).toHaveBeenCalledWith({ index: 2 });
    expect(queueSelection().indices).toEqual([]);
  });

  it('ends the selection on a plain click below the rows, not on a click outside the list', () => {
    renderQueue();
    click(row(1), { ctrlKey: true });

    click(document.body);
    expect(queueSelection().indices).toEqual([1]);

    click(container.firstElementChild as HTMLElement, { ctrlKey: true });
    expect(queueSelection().indices).toEqual([1]);

    click(container.firstElementChild as HTMLElement);
    expect(queueSelection().indices).toEqual([]);
    expect(bindingMocks.playbackPlayQueueIndex).not.toHaveBeenCalled();
  });

  it('on SP plays a tapped row, toggles it while selecting, and opens the row menu only from the ⋮ button', () => {
    platform.current = 'android';
    renderQueue();
    const menuButton = (index: number) => container.querySelectorAll<HTMLElement>('[aria-label="More"]')[index];

    click(row(2));
    expect(bindingMocks.playbackPlayQueueIndex).toHaveBeenCalledWith({ index: 2 });

    toggleQueueSelection(0);
    click(row(1));
    expect(queueSelection().indices).toEqual([0, 1]);
    expect(bindingMocks.playbackPlayQueueIndex).toHaveBeenCalledTimes(1);

    click(menuButton(3));
    expect(menuMocks.showContextMenu).toHaveBeenCalledTimes(1);
    expect(queueSelection().indices).toEqual([0, 1]);
  });

  it('on PC has no ⋮ button', () => {
    renderQueue();

    expect(container.querySelector('[aria-label="More"]')).toBeNull();
  });
});
