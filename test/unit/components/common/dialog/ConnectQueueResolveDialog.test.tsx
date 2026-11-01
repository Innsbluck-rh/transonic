import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ConnectQueueResolveDialog from '~/components/common/dialog/ConnectQueueResolveDialog';
import { setSettingsStore } from '~/features/settings/service';
import { setConnectStore } from '~/stores/ConnectStore';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { resetConnectStore, setSharedPlayback, sharedPlayback } from '~test/fixtures/connect';
import { playbackStatus, resetPlaybackStore, song } from '~test/fixtures/playback';
import { appSettings } from '~test/fixtures/settings';
import { flushPromises } from '~test/helpers/async';

vi.mock('~/bindings', () => ({
  commands: {
    connectResolveQueue: vi.fn(),
    settingsUpdate: vi.fn(),
  },
}));

import { commands } from '~/bindings';

describe('ConnectQueueResolveDialog', () => {
  let container: HTMLDivElement;
  let dispose: (() => void) | undefined;

  beforeEach(() => {
    resetPlaybackStore();
    resetConnectStore();
    setSettingsStore(appSettings({ connect: { enabled: true } }));
    vi.mocked(commands.connectResolveQueue).mockResolvedValue({ status: 'ok', data: null });
    vi.mocked(commands.settingsUpdate).mockImplementation(async (payload) => ({ status: 'ok', data: payload.settings }));

    setSharedPlayback(
      sharedPlayback({ queue: [song('server-a', 'Server A'), song('server-b', 'Server B')], currentIndex: 1, playingState: 'paused' })
    );
    setPlaybackStore(
      'status',
      playbackStatus({ queue: [song('local-a', 'Local A'), song('local-b', 'Local B'), song('local-c', 'Local C')], currentIndex: 0 })
    );

    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    dispose = undefined;
    container.remove();
    vi.clearAllMocks();
  });

  function renderDialog() {
    dispose = render(() => <ConnectQueueResolveDialog />, container);
  }

  function setQueueResolution(pushing: boolean, error: string | null = null) {
    setConnectStore('runtime', 'queueResolution', { pushing, error });
  }

  function button(label: string) {
    const found = Array.from(container.querySelectorAll('button')).find((element) => element.textContent?.includes(label));
    if (!found) {
      throw new Error(`button ${label} not found`);
    }
    return found;
  }

  it('stays hidden while no queue resolution is pending', () => {
    renderDialog();

    expect(container.textContent).toBe('');
  });

  it('shows both queues with their current entries', () => {
    setQueueResolution(false);
    renderDialog();

    const text = container.textContent ?? '';
    expect(text).toContain('2 items / paused');
    expect(text).toContain('Server B');
    expect(text).toContain('3 items / playing');
    expect(text).toContain('Local A');
  });

  it('sends the chosen resolution', async () => {
    setQueueResolution(false);
    renderDialog();

    button('Push Local').click();
    await flushPromises();
    button('Pull From Server').click();
    await flushPromises();

    expect(commands.connectResolveQueue).toHaveBeenNthCalledWith(1, { choice: 'pushLocal' });
    expect(commands.connectResolveQueue).toHaveBeenNthCalledWith(2, { choice: 'pullServer' });
  });

  it('disables the choices while pushing and shows a failed push', () => {
    setQueueResolution(true);
    renderDialog();

    expect(button('Push Local').disabled).toBe(true);
    expect(button('Pull From Server').disabled).toBe(true);

    setQueueResolution(false, 'unsupported playback op "pushState"');

    expect(button('Push Local').disabled).toBe(false);
    expect(container.textContent).toContain('unsupported playback op "pushState"');
  });

  it('shows an error returned by the command', async () => {
    vi.mocked(commands.connectResolveQueue).mockResolvedValue({ status: 'error', error: 'connect: websocket is not connected' });
    setQueueResolution(false);
    renderDialog();

    button('Push Local').click();
    await flushPromises();

    expect(container.textContent).toContain('connect: websocket is not connected');
  });

  it('turns Connect off on cancel', async () => {
    setQueueResolution(false);
    renderDialog();

    button('Cancel').click();
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenCalledWith({
      settings: expect.objectContaining({ connect: expect.objectContaining({ enabled: false }) }),
    });
  });
});
