import { createStore } from 'solid-js/store';

const SETTINGS_STORAGE_KEY = 'transonic.settings.v1';

export interface PlaybackSettings {
  prebufferEnabled: boolean;
}

export interface AppSettings {
  playback: PlaybackSettings;
}

const DEFAULT_PLAYBACK_SETTINGS: PlaybackSettings = {
  prebufferEnabled: false,
};

const DEFAULT_SETTINGS: AppSettings = {
  playback: DEFAULT_PLAYBACK_SETTINGS,
};

export const [settingsStore, setSettingsStore] = createStore<AppSettings>(DEFAULT_SETTINGS);

function normalizePlaybackSettings(raw: Partial<PlaybackSettings> | null | undefined): PlaybackSettings {
  const legacyPrebufferStrategy = raw && 'prebufferStrategy' in raw ? (raw as Partial<{ prebufferStrategy: unknown }>).prebufferStrategy : null;

  return {
    prebufferEnabled: typeof raw?.prebufferEnabled === 'boolean' ? raw.prebufferEnabled : legacyPrebufferStrategy === 'next_track',
  };
}

function normalizeSettings(raw: unknown): AppSettings {
  const candidate = typeof raw === 'object' && raw !== null ? (raw as Partial<AppSettings>) : null;
  return {
    playback: normalizePlaybackSettings(candidate?.playback),
  };
}

function persistSettings() {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settingsStore));
}

export function initializeSettings() {
  if (typeof window === 'undefined') {
    return;
  }

  const raw = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
  if (!raw) {
    return;
  }

  try {
    const parsed = JSON.parse(raw);
    setSettingsStore(normalizeSettings(parsed));
  } catch (error) {
    console.error('failed to parse stored settings', error);
    window.localStorage.removeItem(SETTINGS_STORAGE_KEY);
  }
}

export function setPlaybackSetting<K extends keyof PlaybackSettings>(key: K, value: PlaybackSettings[K]) {
  setSettingsStore('playback', key, value);
  persistSettings();
}
