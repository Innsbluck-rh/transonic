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
        useCustomOutput: false,
        outputDeviceId: null,
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
    window.localStorage.clear();
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

  it('rolls back optimistic output device changes when persistence fails', async () => {
    settingsUpdate.mockResolvedValue({
      status: 'error',
      error: 'write failed',
    });
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    await setPlaybackSetting('outputDeviceId', 'output:speakers');

    expect(consoleErrorSpy).toHaveBeenCalledWith('write failed');
    expect(settingsStore.playback.outputDeviceId).toBeNull();
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

  it('applies Rust-provided settings verbatim without re-normalizing on the TS side', async () => {
    // Rust owns normalization/migration; hydrate must trust its output as-is.
    const settings = {
      appearance: {
        albumDisplayMode: 'list',
      },
      playback: {
        gaplessPlaybackEnabled: true,
        volume: 0.5,
        useCustomOutput: true,
        outputDeviceId: 'output:speakers',
        streamMode: 'transcoding',
        meteredNetworkTranscodingEnabled: true,
        transcodingBitrateLimit: 192,
        useCustomTranscodingCodec: true,
        transcodingCodec: 'opus',
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

    expect(settingsStore).toEqual(settings);
    // No device id generation needed, so no persistence round-trip should occur.
    expect(settingsUpdate).not.toHaveBeenCalled();
  });

  it('forwards raw legacy localStorage settings to Rust for migration on default origin', async () => {
    const legacyBlob = { playback: { prebufferStrategy: 'next_track' } };
    window.localStorage.setItem('transonic.settings.v1', JSON.stringify(legacyBlob));

    const migrated = {
      appearance: {
        albumDisplayMode: 'grid',
      },
      playback: {
        gaplessPlaybackEnabled: true,
        volume: 1,
        useCustomOutput: false,
        outputDeviceId: null,
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
    } as AppSettings;
    settingsUpdate.mockResolvedValue({ status: 'ok', data: migrated });

    const rustDefaults = { ...migrated, connect: { ...migrated.connect, deviceId: 'device-1' } } as AppSettings;
    await hydrateSettings(rustDefaults, 'default');

    // The raw blob is handed to Rust untouched; TS does not normalize it.
    expect(settingsUpdate).toHaveBeenCalledWith({ settings: legacyBlob });
    expect(settingsStore).toEqual(migrated);
    // Migration consumed the legacy key.
    expect(window.localStorage.getItem('transonic.settings.v1')).toBeNull();
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
