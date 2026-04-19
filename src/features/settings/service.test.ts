import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('~/bindings', () => ({
  commands: {
    settingsUpdate: vi.fn(),
  },
}));

import { commands } from '~/bindings';

import { setPlaybackSetting, setSettingsStore, settingsStore } from './service';

describe('settings service', () => {
  const settingsUpdate = vi.mocked(commands.settingsUpdate);

  beforeEach(() => {
    setSettingsStore({
      playback: {
        gaplessPlaybackEnabled: false,
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
});
