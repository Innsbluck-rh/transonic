import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AppSettings } from '~/bindings';
import { appSettings } from '~test/fixtures/settings';

vi.mock('~/bindings', () => ({
  commands: {
    settingsUpdate: vi.fn(),
  },
}));

import { commands } from '~/bindings';

import {
  hydrateSettings,
  setAppearanceSetting,
  setConnectSetting,
  setPlaybackSetting,
  setSettingsStore,
  settingsStore,
} from '~/features/settings/service';

describe('settings service', () => {
  const settingsUpdate = vi.mocked(commands.settingsUpdate);

  beforeEach(() => {
    setSettingsStore(appSettings());
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

  it('applies Rust-provided settings verbatim without re-normalizing on the TS side', async () => {
    // Rust owns normalization/migration; hydrate must trust its output as-is.
    const settings = appSettings({
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
    });
    // A field left out on purpose: hydrate must not fill in TS-side defaults either.
    delete (settings.playback as Partial<AppSettings['playback']>).useAsioOutput;

    await hydrateSettings(settings, 'stored');

    expect(settingsStore).toEqual(settings);
    // No device id generation needed, so no persistence round-trip should occur.
    expect(settingsUpdate).not.toHaveBeenCalled();
  });

  it('forwards raw legacy localStorage settings to Rust for migration on default origin', async () => {
    const legacyBlob = { playback: { prebufferStrategy: 'next_track' } };
    window.localStorage.setItem('transonic.settings.v1', JSON.stringify(legacyBlob));

    const migrated = appSettings({ playback: { gaplessPlaybackEnabled: true } });
    settingsUpdate.mockResolvedValue({ status: 'ok', data: migrated });

    const rustDefaults = appSettings({ playback: { gaplessPlaybackEnabled: true } });
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
