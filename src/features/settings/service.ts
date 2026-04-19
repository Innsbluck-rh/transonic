import { createStore } from 'solid-js/store';
import { commands, type AppSettings, type SettingsOrigin } from '~/bindings';

const SETTINGS_STORAGE_KEY = 'transonic.settings.v1';

const DEFAULT_SETTINGS: AppSettings = {
  playback: {
    gaplessPlaybackEnabled: false,
  },
};

export const [settingsStore, setSettingsStore] = createStore<AppSettings>(DEFAULT_SETTINGS);

function normalizePlaybackSettings(raw: Partial<AppSettings['playback']> | null | undefined): AppSettings['playback'] {
  const legacyGaplessStrategy = raw && 'prebufferStrategy' in raw ? (raw as Partial<{ prebufferStrategy: unknown }>).prebufferStrategy : null;
  const legacyGaplessEnabled = raw && 'prebufferEnabled' in raw ? (raw as Partial<{ prebufferEnabled: unknown }>).prebufferEnabled : null;

  return {
    gaplessPlaybackEnabled:
      typeof raw?.gaplessPlaybackEnabled === 'boolean'
        ? raw.gaplessPlaybackEnabled
        : typeof legacyGaplessEnabled === 'boolean'
          ? legacyGaplessEnabled
          : legacyGaplessStrategy === 'next_track',
  };
}

function normalizeSettings(raw: unknown): AppSettings {
  const candidate = typeof raw === 'object' && raw !== null ? (raw as Partial<AppSettings>) : null;
  return {
    playback: normalizePlaybackSettings(candidate?.playback),
  };
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
  setSettingsStore(settings);

  if (settingsOrigin !== 'default') {
    return;
  }

  const legacySettings = readLegacySettings();
  if (!legacySettings) {
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
}

export async function setPlaybackSetting<K extends keyof AppSettings['playback']>(key: K, value: AppSettings['playback'][K]) {
  const previousSettings: AppSettings = {
    playback: {
      ...settingsStore.playback,
    },
  };
  const nextSettings: AppSettings = {
    playback: {
      ...settingsStore.playback,
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
