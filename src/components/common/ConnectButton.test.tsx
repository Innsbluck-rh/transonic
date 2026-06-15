// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConnectDevicePresence, ConnectSharedPlaybackState, PlayingState } from '~/bindings';
import { setConnectStore } from '~/stores/ConnectStore';
import ConnectButton from './ConnectButton';

vi.mock('@iconify-icon/solid', () => ({
  Icon: (props: { icon: string; class?: string; classList?: Record<string, boolean> }) => (
    <span data-testid='connect-icon' data-icon={props.icon} class={props.class} classList={props.classList} />
  ),
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

function sharedPlayback(activeDeviceId: string | null, playingState: PlayingState): ConnectSharedPlaybackState {
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

function resetConnectStore() {
  setConnectStore({
    status: 'disabled',
    message: null,
    version: null,
    protocolVersion: null,
    capabilities: null,
    runtime: {
      enabled: false,
      connected: false,
      message: null,
      deviceId: null,
      seq: 0,
    },
    devices: [],
    sharedPlayback: null,
    sharedPlaybackReceivedAtMs: null,
  });
}

function setConnectedState(activeDeviceId: string | null, playingState: PlayingState, devices: ConnectDevicePresence[]) {
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
    devices,
    sharedPlayback: sharedPlayback(activeDeviceId, playingState),
    sharedPlaybackReceivedAtMs: 0,
  });
}

describe('ConnectButton', () => {
  let container: HTMLDivElement;
  let dispose: (() => void) | undefined;

  beforeEach(() => {
    resetConnectStore();
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    resetConnectStore();
  });

  it('ignores outside clicks before the menu has been opened', () => {
    dispose = render(() => <ConnectButton />, container);

    expect(() => {
      document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }).not.toThrow();
  });

  it('opens and closes safely from the trigger button', () => {
    dispose = render(() => <ConnectButton />, container);

    const button = container.querySelector('button');
    expect(button).not.toBeNull();

    expect(() => {
      button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }).not.toThrow();
    expect(container.textContent).toContain("there's no devices");

    expect(() => {
      button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }).not.toThrow();
    expect(container.textContent).not.toContain("there's no devices");
  });

  it('closes safely from an outside click after the menu opens', () => {
    dispose = render(() => <ConnectButton />, container);

    const button = container.querySelector('button');
    expect(button).not.toBeNull();

    button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(container.textContent).toContain("there's no devices");

    expect(() => {
      document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }).not.toThrow();
    expect(container.textContent).not.toContain("there's no devices");
  });

  it('uses the volume icon for the active playing device', () => {
    setConnectedState('remote-device', 'playing', [device('remote-device', 'Living Room')]);

    dispose = render(() => <ConnectButton />, container);

    const icon = container.querySelector('[data-testid="connect-icon"]') as HTMLElement | null;
    expect(icon?.dataset.icon).toBe('material-symbols:volume-up');
    expect(container.textContent).toContain('Living Room');
  });

  it('shows the local active device as italic This Device', () => {
    setConnectedState('local-device', 'paused', [device('remote-device', 'Living Room')]);

    dispose = render(() => <ConnectButton />, container);

    const label = container.querySelector('button p') as HTMLElement | null;
    expect(label?.textContent).toBe('This Device');
    expect(label?.classList.contains('italic')).toBe(true);
  });

  it('shows an italic secondary fallback when nothing is active', () => {
    setConnectedState(null, 'idle', [device('remote-device', 'Living Room')]);

    dispose = render(() => <ConnectButton />, container);

    const label = container.querySelector('button p') as HTMLElement | null;
    expect(label?.textContent).toBe('(Nothing Active)');
    expect(label?.classList.contains('italic')).toBe(true);
    expect(label?.classList.contains('text-secondary-text')).toBe(true);
  });
});
