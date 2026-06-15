// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AppSettings } from '~/bindings';
import { setSettingsStore } from '~/features/settings/service';
import { setConnectStore } from '~/stores/ConnectStore';
import ConnectSettings from './ConnectSettings';

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

vi.mock('~/bindings', () => ({
  commands: {
    getDefaultDeviceName: vi.fn().mockResolvedValue('test-device'),
    settingsUpdate: vi.fn(),
    connectGetStateSnapshot: vi.fn(),
  },
  events: {
    connectStateUpdated: {
      listen: vi.fn(),
    },
  },
}));

import { commands } from '~/bindings';

function enabledSettings(): AppSettings {
  return {
    appearance: {
      albumDisplayMode: 'grid',
    },
    playback: {
      gaplessPlaybackEnabled: false,
      volume: 1,
      streamMode: 'raw',
      meteredNetworkTranscodingEnabled: false,
      transcodingBitrateLimit: 320,
      useCustomTranscodingCodec: false,
      transcodingCodec: 'auto',
    },
    connect: {
      enabled: true,
      useSubsonicServerHost: true,
      connectServerPort: 4747,
      connectServerHost: null,
      deviceId: 'device-1',
      deviceName: 'This device',
      allowInsecureConnectServer: false,
    },
  };
}

describe('ConnectSettings', () => {
  let container: HTMLDivElement | undefined;
  let dispose: (() => void) | undefined;

  beforeEach(() => {
    vi.mocked(commands.settingsUpdate).mockImplementation(async (payload) => ({
      status: 'ok',
      data: payload.settings,
    }));
    vi.mocked(commands.connectGetStateSnapshot).mockResolvedValue({
      runtime: {
        enabled: false,
        connected: false,
        message: null,
        serverUrl: null,
        deviceId: null,
        seq: 0,
      },
      devices: [],
      sharedPlayback: null,
    });
  });

  afterEach(() => {
    dispose?.();
    container?.remove();
    vi.clearAllMocks();
  });

  function renderConnectSettings(settings: AppSettings) {
    setSettingsStore(settings);
    setConnectStore({
      status: settings.connect.enabled ? 'available' : 'disabled',
      message: null,
      version: settings.connect.enabled ? '0.1.0' : null,
      protocolVersion: settings.connect.enabled ? 2 : null,
      capabilities: null,
      runtime: {
        enabled: settings.connect.enabled === true,
        connected: settings.connect.enabled === true,
        message: settings.connect.enabled === true ? 'connect: websocket connected' : null,
        deviceId: settings.connect.enabled === true ? 'device-1' : null,
        seq: settings.connect.enabled === true ? 1 : 0,
      },
      devices: [],
      sharedPlayback: null,
      sharedPlaybackReceivedAtMs: null,
    });

    container = document.createElement('div');
    document.body.appendChild(container);
    dispose = render(() => <ConnectSettings />, container);
    return container;
  }

  function flushPromises() {
    return new Promise((resolve) => window.setTimeout(resolve, 0));
  }

  it('persists disabled Connect settings when the main toggle is unchecked', async () => {
    const root = renderConnectSettings(enabledSettings());
    const connectToggle = root.querySelector('input[type="checkbox"]') as HTMLInputElement | null;
    expect(connectToggle).not.toBeNull();

    connectToggle!.checked = false;
    connectToggle!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        connect: expect.objectContaining({
          enabled: false,
        }),
      }),
    });
    expect(commands.connectGetStateSnapshot).toHaveBeenCalled();
    expect(root.textContent).not.toContain('Advanced');
    expect(root.textContent).not.toContain('Use Subsonic Host URL');
    expect(root.textContent).not.toContain('Device Name');
  });

  it('persists enabled Connect settings when the main toggle is checked', async () => {
    const settings = enabledSettings();
    settings.connect.enabled = false;
    const root = renderConnectSettings(settings);
    const connectToggle = root.querySelector('input[type="checkbox"]') as HTMLInputElement | null;
    expect(connectToggle).not.toBeNull();

    connectToggle!.checked = true;
    connectToggle!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        connect: expect.objectContaining({
          enabled: true,
        }),
      }),
    });
    expect(commands.connectGetStateSnapshot).toHaveBeenCalled();
    expect(root.textContent).toContain('Advanced');
  });

  it('enables Advanced Apply only for dirty Connect settings', async () => {
    const root = renderConnectSettings(enabledSettings());
    const applyButton = Array.from(root.querySelectorAll('button')).find((button) => button.textContent?.trim() === 'Apply');
    expect(applyButton).toBeDefined();
    expect(applyButton!.disabled).toBe(true);

    const deviceNameInput = root.querySelector('input[placeholder="This device"]') as HTMLInputElement | null;
    expect(deviceNameInput).not.toBeNull();

    deviceNameInput!.value = 'Living Room';
    deviceNameInput!.dispatchEvent(new Event('input', { bubbles: true }));
    expect(applyButton!.disabled).toBe(false);

    applyButton!.click();
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenCalledWith({
      settings: expect.objectContaining({
        connect: expect.objectContaining({
          enabled: true,
          deviceName: 'Living Room',
        }),
      }),
    });
  });
});
