import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('~/bindings', () => ({
  commands: {
    settingsUpdate: vi.fn(),
  },
}));

import { commands } from '~/bindings';

import { setConnectSetting, setPlaybackSetting, setSettingsStore, settingsStore } from './service';

describe('settings service', () => {
  const settingsUpdate = vi.mocked(commands.settingsUpdate);

  beforeEach(() => {
    setSettingsStore({
      playback: {
        gaplessPlaybackEnabled: false,
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
