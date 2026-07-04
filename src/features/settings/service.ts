import { createStore } from 'solid-js/store';
import { commands, type AppSettings, type SettingsOrigin } from '~/bindings';

const SETTINGS_STORAGE_KEY = 'transonic.settings.v1';

const DEFAULT_SETTINGS: AppSettings = {
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
    deviceId: '',
    deviceName: null,
    allowInsecureConnectServer: false,
  },
};

export const [settingsStore, setSettingsStore] = createStore<AppSettings>(DEFAULT_SETTINGS);

function normalizeAppearanceSettings(raw: Partial<AppSettings['appearance']> | null | undefined): AppSettings['appearance'] {
  return {
    albumDisplayMode: raw?.albumDisplayMode === 'list' ? 'list' : 'grid',
  };
}

function normalizePlaybackSettings(raw: Partial<AppSettings['playback']> | null | undefined): AppSettings['playback'] {
  const legacyGaplessStrategy = raw && 'prebufferStrategy' in raw ? (raw as Partial<{ prebufferStrategy: unknown }>).prebufferStrategy : null;
  const legacyGaplessEnabled = raw && 'prebufferEnabled' in raw ? (raw as Partial<{ prebufferEnabled: unknown }>).prebufferEnabled : null;
  const volume = typeof raw?.volume === 'number' && Number.isFinite(raw.volume) ? Math.min(Math.max(raw.volume, 0), 1) : 1;
  const outputDeviceId = normalizeOutputDeviceId(raw?.outputDeviceId);

  return {
    gaplessPlaybackEnabled:
      typeof raw?.gaplessPlaybackEnabled === 'boolean'
        ? raw.gaplessPlaybackEnabled
        : typeof legacyGaplessEnabled === 'boolean'
          ? legacyGaplessEnabled
          : legacyGaplessStrategy === 'next_track',
    volume,
    useCustomOutput: typeof raw?.useCustomOutput === 'boolean' ? raw.useCustomOutput : outputDeviceId !== null,
    outputDeviceId,
    streamMode: normalizePlaybackStreamMode(raw?.streamMode),
    meteredNetworkTranscodingEnabled: raw?.meteredNetworkTranscodingEnabled === true,
    transcodingBitrateLimit: normalizeTranscodingBitrateLimit(raw?.transcodingBitrateLimit),
    useCustomTranscodingCodec: raw?.useCustomTranscodingCodec === true,
    transcodingCodec: normalizePlaybackTranscodingCodec(raw?.transcodingCodec),
  };
}

function normalizeOutputDeviceId(raw: unknown): AppSettings['playback']['outputDeviceId'] {
  return typeof raw === 'string' && raw.trim().length > 0 ? raw.trim() : null;
}

function normalizePlaybackStreamMode(raw: unknown): AppSettings['playback']['streamMode'] {
  return raw === 'transcoding' ? 'transcoding' : 'raw';
}

function normalizeTranscodingBitrateLimit(raw: unknown): AppSettings['playback']['transcodingBitrateLimit'] {
  if (typeof raw !== 'number' || !Number.isFinite(raw)) {
    return 320;
  }
  const bitrate = Math.trunc(raw);
  return bitrate > 0 ? bitrate : 320;
}

function normalizePlaybackTranscodingCodec(raw: unknown): AppSettings['playback']['transcodingCodec'] {
  switch (raw) {
    case 'mp3':
    case 'flac':
    case 'aac':
    case 'alac':
    case 'vorbis':
    case 'opus':
      return raw;
    default:
      return 'auto';
  }
}

function normalizeSettings(raw: unknown): AppSettings {
  const candidate = typeof raw === 'object' && raw !== null ? (raw as Partial<AppSettings>) : null;
  return {
    appearance: normalizeAppearanceSettings(candidate?.appearance),
    playback: normalizePlaybackSettings(candidate?.playback),
    connect: normalizeConnectSettings(candidate?.connect),
  };
}

function normalizeConnectSettings(raw: Partial<AppSettings['connect']> | null | undefined): AppSettings['connect'] {
  const hasConnectServerHost = typeof raw?.connectServerHost === 'string' && raw.connectServerHost.trim().length > 0;
  const legacyServerUrl = raw && 'serverUrl' in raw && typeof raw.serverUrl === 'string' ? raw.serverUrl : null;
  const splitSource = hasConnectServerHost ? raw.connectServerHost : legacyServerUrl;
  const split = splitUrlHostAndPort(splitSource);
  const rawPort = raw?.connectServerPort;
  const normalizedRawPort = typeof rawPort === 'number' && Number.isInteger(rawPort) && rawPort > 0 && rawPort <= 65535 ? rawPort : null;
  const connectServerPort = hasConnectServerHost ? (normalizedRawPort ?? split.port ?? 4747) : (split.port ?? normalizedRawPort ?? 4747);

  return {
    enabled: typeof raw?.enabled === 'boolean' ? raw.enabled : false,
    useSubsonicServerHost: typeof raw?.useSubsonicServerHost === 'boolean' ? raw.useSubsonicServerHost : true,
    connectServerPort,
    connectServerHost: split.host.length > 0 ? split.host : null,
    deviceId: typeof raw?.deviceId === 'string' ? raw.deviceId : '',
    deviceName: typeof raw?.deviceName === 'string' && raw.deviceName.trim().length > 0 ? raw.deviceName : null,
    allowInsecureConnectServer: typeof raw?.allowInsecureConnectServer === 'boolean' ? raw.allowInsecureConnectServer : false,
  };
}

function splitUrlHostAndPort(rawUrl: string | null | undefined) {
  const trimmed = rawUrl?.trim();
  if (!trimmed) {
    return { host: '', port: null as number | null };
  }

  try {
    const parsed = new URL(trimmed);
    const port = parsed.port ? Number(parsed.port) : null;
    parsed.pathname = '';
    parsed.search = '';
    parsed.hash = '';
    parsed.port = '';
    return {
      host: parsed.toString().replace(/\/$/, ''),
      port: Number.isInteger(port) && port !== null ? port : null,
    };
  } catch {
    return { host: trimmed, port: null };
  }
}

function generateDeviceId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }

  return `device-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}

async function ensureConnectDeviceId(settings: AppSettings) {
  if ((settings.connect.deviceId ?? '').trim().length > 0) {
    return settings;
  }

  const nextSettings: AppSettings = {
    ...settings,
    connect: {
      ...settings.connect,
      deviceId: generateDeviceId(),
    },
  };
  setSettingsStore(nextSettings);
  const result = await commands.settingsUpdate({ settings: nextSettings });
  if (result.status === 'error') {
    console.error(result.error);
    return nextSettings;
  }
  setSettingsStore(result.data);
  return result.data;
}

function readLegacySettings(): AppSettings | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const raw = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
  if (!raw) {
    return null;
  }

  try {
    return normalizeSettings(JSON.parse(raw));
  } catch (error) {
    console.error('failed to parse stored settings', error);
    window.localStorage.removeItem(SETTINGS_STORAGE_KEY);
    return null;
  }
}

function clearLegacySettings() {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.removeItem(SETTINGS_STORAGE_KEY);
}

export async function hydrateSettings(settings: AppSettings, settingsOrigin: SettingsOrigin) {
  const normalizedSettings = normalizeSettings(settings);
  setSettingsStore(normalizedSettings);

  if (settingsOrigin !== 'default') {
    await ensureConnectDeviceId(normalizedSettings);
    return;
  }

  const legacySettings = readLegacySettings();
  if (!legacySettings) {
    await ensureConnectDeviceId(normalizedSettings);
    return;
  }

  setSettingsStore(legacySettings);
  const result = await commands.settingsUpdate({ settings: legacySettings });
  if (result.status === 'error') {
    console.error(result.error);
    return;
  }

  setSettingsStore(result.data);
  clearLegacySettings();
  await ensureConnectDeviceId(result.data);
}

export async function setPlaybackSetting<K extends keyof AppSettings['playback']>(key: K, value: AppSettings['playback'][K]) {
  return setPlaybackSettings({ [key]: value } as Pick<AppSettings['playback'], K>);
}

export async function setPlaybackSettings(patch: Partial<AppSettings['playback']>) {
  const previousSettings: AppSettings = {
    appearance: {
      ...settingsStore.appearance,
    },
    playback: {
      ...settingsStore.playback,
    },
    connect: {
      ...settingsStore.connect,
    },
  };
  const nextSettings: AppSettings = {
    appearance: {
      ...settingsStore.appearance,
    },
    playback: {
      ...settingsStore.playback,
      ...patch,
    },
    connect: {
      ...settingsStore.connect,
    },
  };

  setSettingsStore(nextSettings);
  const result = await commands.settingsUpdate({ settings: nextSettings });
  if (result.status === 'error') {
    console.error(result.error);
    setSettingsStore(previousSettings);
    return;
  }

  setSettingsStore(result.data);
}

export async function setAppearanceSetting<K extends keyof AppSettings['appearance']>(key: K, value: AppSettings['appearance'][K]) {
  const previousSettings: AppSettings = {
    appearance: {
      ...settingsStore.appearance,
    },
    playback: {
      ...settingsStore.playback,
    },
    connect: {
      ...settingsStore.connect,
    },
  };
  const nextSettings: AppSettings = {
    appearance: {
      ...settingsStore.appearance,
      [key]: value,
    },
    playback: {
      ...settingsStore.playback,
    },
    connect: {
      ...settingsStore.connect,
    },
  };

  setSettingsStore(nextSettings);
  const result = await commands.settingsUpdate({ settings: nextSettings });
  if (result.status === 'error') {
    console.error(result.error);
    setSettingsStore(previousSettings);
    return;
  }

  setSettingsStore(result.data);
}

export async function setConnectSetting<K extends keyof AppSettings['connect']>(key: K, value: AppSettings['connect'][K]) {
  const previousSettings: AppSettings = {
    appearance: {
      ...settingsStore.appearance,
    },
    playback: {
      ...settingsStore.playback,
    },
    connect: {
      ...settingsStore.connect,
    },
  };
  const nextSettings: AppSettings = {
    appearance: {
      ...settingsStore.appearance,
    },
    playback: {
      ...settingsStore.playback,
    },
    connect: {
      ...settingsStore.connect,
      [key]: value,
    },
  };

  setSettingsStore(nextSettings);
  const result = await commands.settingsUpdate({ settings: nextSettings });
  if (result.status === 'error') {
    console.error(result.error);
    setSettingsStore(previousSettings);
    return;
  }

  setSettingsStore(result.data);
}

export async function setConnectSettings(connect: AppSettings['connect']) {
  const previousSettings: AppSettings = {
    appearance: {
      ...settingsStore.appearance,
    },
    playback: {
      ...settingsStore.playback,
    },
    connect: {
      ...settingsStore.connect,
    },
  };
  const nextSettings: AppSettings = {
    appearance: {
      ...settingsStore.appearance,
    },
    playback: {
      ...settingsStore.playback,
    },
    connect: normalizeConnectSettings(connect),
  };

  setSettingsStore(nextSettings);
  const result = await commands.settingsUpdate({ settings: nextSettings });
  if (result.status === 'error') {
    console.error(result.error);
    setSettingsStore(previousSettings);
    return { status: 'error' as const, error: result.error };
  }

  setSettingsStore(result.data);
  return { status: 'ok' as const, data: result.data };
}
