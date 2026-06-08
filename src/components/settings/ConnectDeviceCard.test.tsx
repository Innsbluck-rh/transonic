// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConnectDevicePresence, ConnectSharedPlaybackState, PlayingState } from '~/bindings';
import { setConnectStore } from '~/stores/ConnectStore';
import ConnectDeviceCard from './ConnectDeviceCard';

vi.mock('@iconify-icon/solid', () => ({
  Icon: (props: { icon: string; class?: string; classList?: Record<string, boolean> }) => (
    <span data-testid='device-icon' data-icon={props.icon} class={props.class} classList={props.classList} />
  ),
}));

vi.mock('~/bindings', () => ({
  commands: {
    connectTransferPlayback: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
  },
}));

function device(deviceId: string, displayName = deviceId): ConnectDevicePresence {
  return {
    deviceId,
    displayName,
    platform: 'test',
    appVersion: '0.0.0',
    lastSeenAt: '2026-05-25T00:00:00Z',
    online: true,
  };
}

function sharedPlayback(activeDeviceId: string, playingState: PlayingState): ConnectSharedPlaybackState {
  return {
    seq: 1,
    activeDeviceId,
    state: {
      playingState,
      queue: [],
      currentIndex: null,
      playNextQueueLen: 0,
      currentPositionMs: 0,
      currentSongId: null,
    },
    updatedAt: '2026-05-25T00:00:00Z',
    updatedByDeviceId: activeDeviceId,
    updateReason: 'snapshot',
  };
}

function setConnectDeviceState(activeDeviceId: string, playingState: PlayingState) {
  setConnectStore({
    status: 'available',
    message: null,
    version: '0.1.0',
    protocolVersion: 2,
    capabilities: {
      presence: true,
      sharedQueue: true,
      sharedPlayback: true,
      playbackTransfer: true,
    },
    runtime: {
      enabled: true,
      connected: true,
      message: null,
      deviceId: 'local-device',
      seq: 1,
    },
    devices: [device('active-device', 'Living Room')],
    sharedPlayback: sharedPlayback(activeDeviceId, playingState),
    sharedPlaybackReceivedAtMs: 0,
  });
}

describe('ConnectDeviceCard', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    vi.clearAllMocks();
  });

  it('highlights the active device even when playback is paused', () => {
    setConnectDeviceState('active-device', 'paused');

    dispose = render(() => <ConnectDeviceCard device={device('active-device', 'Living Room')} />, container);

    const icon = container.querySelector('[data-testid="device-icon"]') as HTMLElement | null;
    const label = container.querySelector('p') as HTMLElement | null;
    expect(icon?.dataset.icon).toBe('material-symbols:devices');
    expect(icon?.classList.contains('text-accent')).toBe(true);
    expect(label?.classList.contains('text-accent')).toBe(true);
  });

  it('uses the volume icon only for the active playing device', () => {
    setConnectDeviceState('active-device', 'playing');

    dispose = render(() => <ConnectDeviceCard device={device('active-device', 'Living Room')} />, container);

    const icon = container.querySelector('[data-testid="device-icon"]') as HTMLElement | null;
    expect(icon?.dataset.icon).toBe('material-symbols:volume-up');
    expect(icon?.classList.contains('text-accent')).toBe(true);
  });
});
