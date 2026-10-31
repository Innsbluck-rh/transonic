import type { AppSettings } from '~/bindings';

interface AppSettingsOverrides {
  appearance?: Partial<AppSettings['appearance']>;
  playback?: Partial<AppSettings['playback']>;
  connect?: Partial<AppSettings['connect']>;
}

export function appSettings(overrides: AppSettingsOverrides = {}): AppSettings {
  return {
    appearance: {
      albumDisplayMode: 'grid',
      ...overrides.appearance,
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
      ...overrides.playback,
    },
    connect: {
      enabled: false,
      useSubsonicServerHost: true,
      connectServerPort: 4747,
      connectServerHost: null,
      deviceId: 'device-1',
      deviceName: null,
      allowInsecureConnectServer: false,
      ...overrides.connect,
    },
  };
}
