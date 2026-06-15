import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AppSettings } from '~/bindings';

vi.mock('~/bindings', () => ({
  commands: {
    settingsUpdate: vi.fn(),
  },
}));

import { commands } from '~/bindings';

import { hydrateSettings, setAppearanceSetting, setConnectSetting, setPlaybackSetting, setSettingsStore, settingsStore } from './service';

describe('settings service', () => {
  const settingsUpdate = vi.mocked(commands.settingsUpdate);

  beforeEach(() => {
    setSettingsStore({
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
        enabled: false,
        useSubsonicServerHost: true,
        connectServerPort: 4747,
        connectServerHost: null,
        deviceId: 'device-1',
        deviceName: null,
        allowInsecureConnectServer: false,
      },
    });
    settingsUpdate.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('rolls back optimistic playback setting changes when persistence fails', async () => {
    settingsUpdate.mockResolvedValue({
      status: 'error',
      error: 'write failed',
    });
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    await setPlaybackSetting('gaplessPlaybackEnabled', true);

    expect(consoleErrorSpy).toHaveBeenCalledWith('write failed');
    expect(settingsStore.playback.gaplessPlaybackEnabled).toBe(false);
  });

  it('rolls back optimistic playback stream setting changes when persistence fails', async () => {
    settingsUpdate.mockResolvedValue({
      status: 'error',
      error: 'write failed',
    });
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    await setPlaybackSetting('streamMode', 'transcoding');

    expect(consoleErrorSpy).toHaveBeenCalledWith('write failed');
    expect(settingsStore.playback.streamMode).toBe('raw');
  });

  it('rolls back optimistic metered network transcoding changes when persistence fails', async () => {
    settingsUpdate.mockResolvedValue({
      status: 'error',
      error: 'write failed',
    });
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    await setPlaybackSetting('meteredNetworkTranscodingEnabled', true);

    expect(consoleErrorSpy).toHaveBeenCalledWith('write failed');
    expect(settingsStore.playback.meteredNetworkTranscodingEnabled).toBe(false);
  });

  it('normalizes missing metered network transcoding settings to disabled', async () => {
    const settings = {
      appearance: {
        albumDisplayMode: 'grid',
      },
      playback: {
        gaplessPlaybackEnabled: false,
        volume: 1,
        streamMode: 'raw',
        transcodingBitrateLimit: 320,
        useCustomTranscodingCodec: false,
        transcodingCodec: 'auto',
      },
      connect: {
        enabled: false,
        useSubsonicServerHost: true,
        connectServerPort: 4747,
        connectServerHost: null,
        deviceId: 'device-1',
        deviceName: null,
        allowInsecureConnectServer: false,
      },
    } as AppSettings;

    await hydrateSettings(settings, 'stored');

    expect(settingsStore.playback.meteredNetworkTranscodingEnabled).toBe(false);
  });

  it('rolls back optimistic appearance setting changes when persistence fails', async () => {
    settingsUpdate.mockResolvedValue({
      status: 'error',
      error: 'write failed',
    });
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    await setAppearanceSetting('albumDisplayMode', 'list');

    expect(consoleErrorSpy).toHaveBeenCalledWith('write failed');
    expect(settingsStore.appearance.albumDisplayMode).toBe('grid');
  });

  it('rolls back optimistic connect setting changes when persistence fails', async () => {
    settingsUpdate.mockResolvedValue({
      status: 'error',
      error: 'write failed',
    });
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    await setConnectSetting('connectServerHost', 'https://connect.example');

    expect(consoleErrorSpy).toHaveBeenCalledWith('write failed');
    expect(settingsStore.connect.connectServerHost).toBeNull();
  });
});
