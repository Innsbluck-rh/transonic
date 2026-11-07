import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import QueueList from '~/components/common/list/song/QueueList';
import { LONG_PRESS_MS } from '~/components/common/list/song/queueListGestures';
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

  it('on SP a long-press only turns the row on, and the click that follows it does nothing', () => {
    vi.useFakeTimers();
    try {
      platform.current = 'android';
      renderQueue();
      const longPress = (index: number) => {
        const event = new MouseEvent('pointerdown', { bubbles: true, clientX: 10, clientY: 10 });
        Object.defineProperties(event, { pointerId: { value: index + 1 }, pointerType: { value: 'touch' } });
        row(index).dispatchEvent(event);
        vi.advanceTimersByTime(LONG_PRESS_MS);
        click(row(index));
      };

      longPress(1);
      expect(queueSelection().indices).toEqual([1]);

      longPress(3);
      expect(queueSelection().indices).toEqual([1, 3]);
      expect(bindingMocks.playbackPlayQueueIndex).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it('on PC has no ⋮ button', () => {
    renderQueue();

    expect(container.querySelector('[aria-label="More"]')).toBeNull();
  });

  describe('dragging rows', () => {
    const ROW_PX = 50;
    const rowWrapper = (index: number) => container.querySelector<HTMLElement>(`[data-queue-index="${index}"]`)!;
    const preview = () => container.querySelector<HTMLElement>('[data-queue-move-preview]');
    const pointer = (type: string, target: EventTarget, clientY: number) => {
      const event = new MouseEvent(type, { bubbles: true, button: 0, clientX: 10, clientY });
      Object.defineProperty(event, 'pointerId', { value: 1 });
      target.dispatchEvent(event);
    };
    /** Drags row `from` to `clientY`, rows being ROW_PX tall from the top. */
    const dragTo = (from: number, clientY: number) => {
      pointer('pointerdown', row(from), from * ROW_PX + 10);
      pointer('pointermove', window, clientY);
    };

    let boxSpy: { mockRestore: () => void } | undefined;

    beforeEach(() => {
      // jsdom has no layout: each row reports its box from its queue index.
      boxSpy = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
        const index = Number(this.dataset.queueIndex ?? 0);
        return {
          top: index * ROW_PX,
          height: ROW_PX,
          bottom: (index + 1) * ROW_PX,
          left: 0,
          right: 300,
          width: 300,
          x: 0,
          y: index * ROW_PX,
        } as DOMRect;
      });
    });

    afterEach(() => {
      boxSpy?.mockRestore();
    });

    it('fades the dragged row and draws it inserted where it would land, keeping it there until the moved queue arrives', async () => {
      const { commands } = await import('~/bindings');
      setPlaybackStore('status', playbackStatus({ queue, currentIndex: 0 }));
      const [items, setItems] = createSignal(queue);
      dispose = render(() => <QueueList queue={items()} />, container);

      dragTo(0, 2.6 * ROW_PX);

      expect(rowWrapper(0).classList.contains('opacity-25')).toBe(true);
      expect(rowWrapper(1).classList.contains('opacity-25')).toBe(false);
      expect(preview()?.nextElementSibling).toBe(rowWrapper(3));
      expect(preview()?.querySelector('.song-item-title')?.textContent).toBe('a');

      pointer('pointerup', window, 2.6 * ROW_PX);
      expect(commands.playbackMoveQueueIndices).toHaveBeenCalledWith({ indices: [0], insertBefore: 3 });
      expect(preview()).not.toBeNull();

      setItems([queue[1], queue[2], queue[0], queue[3]]);
      expect(preview()).toBeNull();
      expect(rowWrapper(0).classList.contains('opacity-25')).toBe(false);
    });

    it('draws several selected rows as one group row, after the last row when dragged past it', () => {
      renderQueue();
      toggleQueueSelection(0);
      toggleQueueSelection(2);

      dragTo(0, 10 * ROW_PX);

      expect(rowWrapper(0).classList.contains('opacity-25')).toBe(true);
      expect(rowWrapper(2).classList.contains('opacity-25')).toBe(true);
      expect(preview()?.previousElementSibling).toBe(rowWrapper(3));
      expect(preview()?.textContent).toContain('[2 items]');
    });

    it('draws nothing inserted, and moves nothing, where letting go would not move the rows', async () => {
      const { commands } = await import('~/bindings');
      renderQueue();

      dragTo(1, 1.4 * ROW_PX);
      expect(rowWrapper(1).classList.contains('opacity-25')).toBe(true);
      expect(preview()).toBeNull();

      pointer('pointerup', window, 1.4 * ROW_PX);
      expect(commands.playbackMoveQueueIndices).not.toHaveBeenCalled();
      expect(rowWrapper(1).classList.contains('opacity-25')).toBe(false);
    });
  });
});
