// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AppSettings } from '~/bindings';
import { setSettingsStore } from '~/features/settings/service';
import PlaybackSettings from './PlaybackSettings';

const osMocks = vi.hoisted(() => ({
  platform: vi.fn(() => 'android'),
}));

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

vi.mock('@tauri-apps/plugin-os', () => ({
  platform: osMocks.platform,
}));

vi.mock('~/bindings', () => ({
  commands: {
    settingsUpdate: vi.fn(),
  },
}));

function settings(overrides: Partial<AppSettings['playback']> = {}): AppSettings {
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
      ...overrides,
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
  };
}

describe('PlaybackSettings', () => {
  let container: HTMLDivElement;
  let dispose: (() => void) | undefined;

  beforeEach(() => {
    osMocks.platform.mockReturnValue('android');
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    vi.clearAllMocks();
  });

  function renderPlaybackSettings(appSettings: AppSettings) {
    setSettingsStore(appSettings);
    dispose = render(() => <PlaybackSettings />, container);
    return container;
  }

  it('shows metered network transcoding only on Android raw stream settings', () => {
    const androidRoot = renderPlaybackSettings(settings());
    expect(androidRoot.textContent).toContain('Use Transcoding Stream on metered networks');

    dispose?.();
    dispose = undefined;
    container.textContent = '';
    osMocks.platform.mockReturnValue('windows');

    const windowsRoot = renderPlaybackSettings(settings());
    expect(windowsRoot.textContent).not.toContain('Use Transcoding Stream on metered networks');
  });

  it('shows transcoding details when metered network transcoding is enabled', () => {
    const root = renderPlaybackSettings(
      settings({
        streamMode: 'raw',
        meteredNetworkTranscodingEnabled: true,
      })
    );

    expect(root.textContent).toContain('Stream Bitrate Limit');
    expect(root.textContent).toContain('Transcoding Codec');
  });
});
