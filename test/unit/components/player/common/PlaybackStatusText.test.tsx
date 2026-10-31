import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
// The ASIO marker links to the playback settings, and resolving that route asks
// the OS plugin whether this is a smartphone layout.
vi.mock('@tauri-apps/plugin-os', () => ({
  platform: () => 'windows',
}));

import PlaybackStatusText from '~/components/player/common/PlaybackStatusText';
import { setPlaybackStore } from '~/stores/PlaybackStore';
import { resetConnectStore, setSharedPlayback, sharedPlayback } from '~test/fixtures/connect';
import { actualStreamInfo, playbackStatus, resetPlaybackStore } from '~test/fixtures/playback';

describe('PlaybackStatusText', () => {
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
  });

  it('hides local stream info while another Connect device is active', () => {
    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('local-codec') }));
    setSharedPlayback(sharedPlayback({ activeDeviceId: 'remote-device' }));

    dispose = render(() => <PlaybackStatusText />, container);

    expect(container.textContent).not.toContain('local-codec');
    expect(container.textContent).toBe('');
  });

  it('marks ASIO output and leaves other output paths unmarked', () => {
    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('flac') }));
    dispose = render(() => <PlaybackStatusText />, container);
    expect(container.textContent).toContain('flac');
    expect(container.textContent).not.toContain('ASIO');

    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('flac'), outputHost: 'wasapi' }));
    expect(container.textContent).not.toContain('ASIO');

    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('flac'), outputHost: 'asio' }));
    const asioLabel = Array.from(container.querySelectorAll('*')).find((element) => element.textContent === 'ASIO');
    expect(asioLabel).toBeDefined();
    expect(asioLabel!.className).toContain('text-accent');
    expect(asioLabel!.className).toContain('font-bold');
    // The marker doubles as the way to reach the setting that produced it.
    expect(asioLabel!.getAttribute('href')).toBe('/settings#playback');
  });

  it('does not mark ASIO output while another Connect device is active', () => {
    // The output path is a fact about this machine, so it must not be claimed
    // for audio that is coming out of a different device.
    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('flac'), outputHost: 'asio' }));
    setSharedPlayback(sharedPlayback({ activeDeviceId: 'remote-device' }));

    dispose = render(() => <PlaybackStatusText />, container);

    expect(container.textContent).not.toContain('ASIO');
  });

  it('shows local stream info when this Connect device is active', () => {
    setPlaybackStore('status', playbackStatus({ actualStreamInfo: actualStreamInfo('local-codec') }));
    setSharedPlayback(sharedPlayback({ activeDeviceId: 'own-device' }));

    dispose = render(() => <PlaybackStatusText />, container);

    expect(container.textContent).toContain('local-codec');
  });
});
