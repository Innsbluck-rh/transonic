import { createStore } from 'solid-js/store';
import { commands, type AppSettings, type SettingsOrigin } from '~/bindings';

// Pre-Rust settings blob persisted by very old builds directly in localStorage.
// Migration/normalization of its contents is Rust's job (see `readRawLegacySettings`).
const LEGACY_SETTINGS_STORAGE_KEY = 'transonic.settings.v1';

// Pre-hydration placeholder only. Rust is the single source of truth for
// defaults/normalization/migration (`src-tauri/src/models/settings.rs`,
// `src-tauri/src/app_settings.rs`); `hydrateSettings` overwrites this store with
// Rust-provided values at startup. Keep these in sync with the Rust `Default` impls,
// but never re-derive or normalize settings on the TS side.
const DEFAULT_SETTINGS: AppSettings = {
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
    deviceId: '',
    deviceName: null,
    allowInsecureConnectServer: false,
  },
};

export const [settingsStore, setSettingsStore] = createStore<AppSettings>(DEFAULT_SETTINGS);

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

// Returns the raw parsed legacy blob (unknown shape) so Rust can migrate/normalize it.
// The TS side intentionally does not normalize. Returns null when absent or unparseable.
function readRawLegacySettings(): unknown {
  if (typeof window === 'undefined') {
    return null;
  }

  const raw = window.localStorage.getItem(LEGACY_SETTINGS_STORAGE_KEY);
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw);
  } catch (error) {
    console.error('failed to parse stored settings', error);
    window.localStorage.removeItem(LEGACY_SETTINGS_STORAGE_KEY);
    return null;
  }
}

function clearLegacySettings() {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.removeItem(LEGACY_SETTINGS_STORAGE_KEY);
}

export async function hydrateSettings(settings: AppSettings, settingsOrigin: SettingsOrigin) {
  // `settings` is already normalized/migrated by Rust; apply it verbatim.
  setSettingsStore(settings);

  if (settingsOrigin !== 'default') {
    await ensureConnectDeviceId(settings);
    return;
  }

  const legacySettings = readRawLegacySettings();
  if (legacySettings === null) {
    await ensureConnectDeviceId(settings);
    return;
  }

  // Hand the raw legacy blob to Rust, which owns migration + normalization + persistence.
  const result = await commands.settingsUpdate({ settings: legacySettings as AppSettings });
  if (result.status === 'error') {
    console.error(result.error);
    await ensureConnectDeviceId(settings);
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
    // Sent as-is; Rust normalizes and returns the canonical value in `result.data`.
    connect,
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
