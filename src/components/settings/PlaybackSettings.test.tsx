// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type AppSettings } from '~/bindings';
import { setSettingsStore } from '~/features/settings/service';
import { setSessionStore } from '~/stores/SessionStore';
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
    playbackGetOutputDevices: vi.fn(),
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
      useCustomOutput: false,
      useAsioOutput: false,
      outputDeviceId: null,
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
    vi.mocked(commands.settingsUpdate).mockImplementation(async (payload) => ({
      status: 'ok',
      data: payload.settings,
    }));
    vi.mocked(commands.playbackGetOutputDevices).mockResolvedValue({
      status: 'ok',
      data: [],
    });
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: false,
    });
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

  function flushPromises() {
    return new Promise((resolve) => window.setTimeout(resolve, 0));
  }

  function findCustomOutputSelect(root: HTMLElement) {
    return Array.from(root.querySelectorAll('select')).find((select) =>
      Array.from(select.options).some((option) => option.textContent === 'System Default' || option.value.startsWith('output:'))
    );
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

  it('shows Windows output devices and persists custom output settings', async () => {
    osMocks.platform.mockReturnValue('windows');
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: true,
    });
    vi.mocked(commands.playbackGetOutputDevices).mockResolvedValue({
      status: 'ok',
      data: [
        {
          id: 'output:speakers',
          name: 'Speakers (Realtek Audio)',
          deviceName: 'Realtek Audio',
          host: 'wasapi',
        },
        {
          id: 'output:headphones',
          name: 'Headphones',
          deviceName: 'USB DAC',
          host: 'wasapi',
        },
      ],
    });

    const root = renderPlaybackSettings(settings());
    await flushPromises();
    const useCustomOutput = root.querySelector<HTMLInputElement>('input[type="checkbox"]');

    expect(findCustomOutputSelect(root)).toBeUndefined();

    useCustomOutput!.checked = true;
    useCustomOutput!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();
    await flushPromises();

    const outputSelect = findCustomOutputSelect(root);
    expect(outputSelect).toBeDefined();
    expect(root.textContent).toContain('System Default');
    expect(outputSelect!.options[1]?.textContent).toBe('Speakers (Realtek Audio)');
    expect(commands.settingsUpdate).toHaveBeenLastCalledWith({
      settings: expect.objectContaining({
        playback: expect.objectContaining({
          useCustomOutput: true,
          outputDeviceId: null,
        }),
      }),
    });

    outputSelect!.value = 'output:headphones';
    outputSelect!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenLastCalledWith({
      settings: expect.objectContaining({
        playback: expect.objectContaining({
          useCustomOutput: true,
          outputDeviceId: 'output:headphones',
        }),
      }),
    });

    useCustomOutput!.checked = false;
    useCustomOutput!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenLastCalledWith({
      settings: expect.objectContaining({
        playback: expect.objectContaining({
          useCustomOutput: false,
          outputDeviceId: null,
        }),
      }),
    });
    expect(findCustomOutputSelect(root)).toBeUndefined();
  });

  it('offers ASIO devices only once ASIO output is enabled and labels them', async () => {
    osMocks.platform.mockReturnValue('windows');
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: true,
      asioOutput: true,
    });
    // Mirrors the backend, which leaves ASIO devices out unless the request asks
    // for them. A mock that always returns them would pass even if enabling ASIO
    // never triggered a refetch, which is exactly the bug this covers.
    vi.mocked(commands.playbackGetOutputDevices).mockImplementation(async (includeAsio) => ({
      status: 'ok',
      data: [
        {
          id: 'wasapi:speakers',
          name: 'Speakers (Realtek Audio)',
          deviceName: 'Realtek Audio',
          host: 'wasapi' as const,
        },
        ...(includeAsio
          ? [
              {
                id: 'asio:Audient USB Audio ASIO Driver',
                name: 'Audient USB Audio ASIO Driver',
                deviceName: 'Audient USB Audio ASIO Driver',
                host: 'asio' as const,
              },
            ]
          : []),
      ],
    }));

    const root = renderPlaybackSettings(settings({ useCustomOutput: true }));
    await flushPromises();
    await flushPromises();

    const optionLabels = () => Array.from(findCustomOutputSelect(root)!.options).map((option) => option.textContent);
    expect(optionLabels()).toEqual(['System Default', 'Speakers (Realtek Audio)']);

    const asioCheckbox = Array.from(root.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')).find((input) =>
      input.parentElement?.textContent?.includes('Use ASIO Output')
    );
    expect(asioCheckbox).toBeDefined();

    asioCheckbox!.checked = true;
    asioCheckbox!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenLastCalledWith({
      settings: expect.objectContaining({
        playback: expect.objectContaining({
          useAsioOutput: true,
          outputDeviceId: null,
        }),
      }),
    });
    expect(optionLabels()).toEqual(['System Default', 'Speakers (Realtek Audio)', 'ASIO: Audient USB Audio ASIO Driver']);

    // Turning it back off has to drop them again, which likewise needs a refetch.
    asioCheckbox!.checked = false;
    asioCheckbox!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();
    await flushPromises();

    expect(optionLabels()).toEqual(['System Default', 'Speakers (Realtek Audio)']);
  });

  it('resets ASIO output when custom output is switched off', async () => {
    osMocks.platform.mockReturnValue('windows');
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: true,
      asioOutput: true,
    });

    const root = renderPlaybackSettings(settings({ useCustomOutput: true, useAsioOutput: true }));
    await flushPromises();

    const useCustomOutput = root.querySelector<HTMLInputElement>('input[type="checkbox"]');
    useCustomOutput!.checked = false;
    useCustomOutput!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenLastCalledWith({
      settings: expect.objectContaining({
        playback: expect.objectContaining({
          useCustomOutput: false,
          useAsioOutput: false,
          outputDeviceId: null,
        }),
      }),
    });
  });

  it('hides the ASIO output toggle when the build has no ASIO support', async () => {
    osMocks.platform.mockReturnValue('windows');
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: true,
      asioOutput: false,
    });

    const root = renderPlaybackSettings(settings({ useCustomOutput: true }));
    await flushPromises();

    expect(root.textContent).toContain('Use Custom Output');
    expect(root.textContent).not.toContain('Use ASIO Output');
  });

  it('keeps the custom output checkbox interactive while devices are loading', async () => {
    osMocks.platform.mockReturnValue('windows');
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: true,
    });
    vi.mocked(commands.playbackGetOutputDevices).mockImplementation(() => new Promise(() => undefined));

    const root = renderPlaybackSettings(settings());
    const useCustomOutput = root.querySelector<HTMLInputElement>('input[type="checkbox"]');

    expect(useCustomOutput).toBeDefined();
    expect(useCustomOutput!.disabled).toBe(false);
    expect(commands.playbackGetOutputDevices).not.toHaveBeenCalled();

    useCustomOutput!.checked = true;
    useCustomOutput!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();

    expect(useCustomOutput!.disabled).toBe(false);
    expect(commands.playbackGetOutputDevices).toHaveBeenCalledTimes(1);
  });

  it('keeps an unavailable saved Windows output device selectable', async () => {
    osMocks.platform.mockReturnValue('windows');
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: true,
    });

    const root = renderPlaybackSettings(settings({ useCustomOutput: true, outputDeviceId: 'output:missing' }));
    await flushPromises();
    const outputSelect = findCustomOutputSelect(root);

    expect(root.textContent).toContain('Unavailable output');
    expect(outputSelect).toBeDefined();
    expect(outputSelect!.value).toBe('output:missing');
  });

  it('resets unavailable custom output to system default when toggled', async () => {
    osMocks.platform.mockReturnValue('windows');
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: true,
    });

    const root = renderPlaybackSettings(settings({ useCustomOutput: true, outputDeviceId: 'output:missing' }));
    await flushPromises();
    const useCustomOutput = root.querySelector<HTMLInputElement>('input[type="checkbox"]');

    useCustomOutput!.checked = false;
    useCustomOutput!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();

    expect(commands.settingsUpdate).toHaveBeenLastCalledWith({
      settings: expect.objectContaining({
        playback: expect.objectContaining({
          useCustomOutput: false,
          outputDeviceId: null,
        }),
      }),
    });
    expect(findCustomOutputSelect(root)).toBeUndefined();

    useCustomOutput!.checked = true;
    useCustomOutput!.dispatchEvent(new Event('change', { bubbles: true }));
    await flushPromises();

    const outputSelect = findCustomOutputSelect(root);
    expect(outputSelect).toBeDefined();
    expect(outputSelect!.value).toBe('');
    expect(outputSelect!.selectedOptions[0]?.textContent).toBe('System Default');
    expect(commands.settingsUpdate).toHaveBeenLastCalledWith({
      settings: expect.objectContaining({
        playback: expect.objectContaining({
          useCustomOutput: true,
          outputDeviceId: null,
        }),
      }),
    });
  });

  it('selects an unavailable Windows output device after settings hydrate', async () => {
    osMocks.platform.mockReturnValue('windows');

    const root = renderPlaybackSettings(settings());
    setSettingsStore(settings({ useCustomOutput: true, outputDeviceId: 'output:missing-device' }));
    setSessionStore('playbackCapabilities', {
      gaplessPlayback: false,
      transcodingCodecs: ['mp3'],
      outputDeviceSelection: true,
    });
    await flushPromises();
    const outputSelect = findCustomOutputSelect(root);

    expect(root.textContent).toContain('Unavailable output');
    expect(outputSelect).toBeDefined();
    expect(outputSelect!.value).toBe('output:missing-device');
    expect(outputSelect!.selectedOptions[0]?.textContent).toBe('Unavailable output');
  });
});
