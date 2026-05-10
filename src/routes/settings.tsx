import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { commands, type CoverArtCacheStatus } from '~/bindings';
import Heading1 from '~/components/common/Heading1';
import Heading2 from '~/components/common/Heading2';
import {
  connectServerHostFromSubsonic,
  refreshConnectRuntimeStatus,
  refreshConnectServerStatus,
  resolveConnectServerUrl,
  splitUrlHostAndPort,
  startConnectDevicesSync,
} from '~/features/connect/service';
import { loadBootstrapToStore } from '~/features/session/service';
import { setConnectSettings, setPlaybackSetting, settingsStore } from '~/features/settings/service';
import { applyTheme, currentTheme } from '~/features/theme/service';
import { connectStore } from '~/stores/ConnectStore';
import { sessionStore } from '~/stores/SessionStore';

const themeOptions = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
] as const;

function formatPosition(ms: number) {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

function currentSongLabel(playback: NonNullable<(typeof connectStore.devices)[number]['playback']>) {
  const index = playback.state.currentIndex;
  if (index !== null) {
    const song = playback.state.queue[index];
    if (song?.title) {
      return song.artist ? `${song.title} - ${song.artist}` : song.title;
    }
  }
  return playback.state.currentSongId ?? 'No song';
}

function normalizePort(value: string) {
  const port = Number(value);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    return 4747;
  }
  return port;
}

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }

  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function SettingsRoute() {
  const activeSession = () => sessionStore.activeSession;
  const isGaplessEnabled = () => settingsStore.playback.gaplessPlaybackEnabled;
  const supportsGaplessPlayback = () => sessionStore.playbackCapabilities.gaplessPlayback;
  const [connectEnabled, setConnectEnabled] = createSignal(settingsStore.connect.enabled === true);
  const [useSubsonicServerHost, setUseSubsonicServerHost] = createSignal(settingsStore.connect.useSubsonicServerHost !== false);
  const [connectServerPort, setConnectServerPort] = createSignal(String(settingsStore.connect.connectServerPort || 4747));
  const [connectServerHost, setConnectServerHost] = createSignal(settingsStore.connect.connectServerHost ?? '');
  const [connectDeviceName, setConnectDeviceName] = createSignal(settingsStore.connect.deviceName ?? '');
  const [allowInsecureConnectServer, setAllowInsecureConnectServer] = createSignal(settingsStore.connect.allowInsecureConnectServer === true);
  const [connectActionError, setConnectActionError] = createSignal<string | null>(null);
  const [connectActionBusy, setConnectActionBusy] = createSignal(false);
  const [backupMessage, setBackupMessage] = createSignal<string | null>(null);
  const [backupBusy, setBackupBusy] = createSignal(false);
  const [coverArtCacheStatus, setCoverArtCacheStatus] = createSignal<CoverArtCacheStatus | null>(null);
  const [coverArtCacheMessage, setCoverArtCacheMessage] = createSignal<string | null>(null);
  const [coverArtCacheBusy, setCoverArtCacheBusy] = createSignal(false);
  const syncConnectForm = () => {
    setConnectEnabled(settingsStore.connect.enabled === true);
    setUseSubsonicServerHost(settingsStore.connect.useSubsonicServerHost !== false);
    setConnectServerPort(String(settingsStore.connect.connectServerPort || 4747));
    setConnectServerHost(settingsStore.connect.connectServerHost ?? '');
    setConnectDeviceName(settingsStore.connect.deviceName ?? '');
    setAllowInsecureConnectServer(settingsStore.connect.allowInsecureConnectServer === true);
  };
  const subsonicConnectServerHost = createMemo(() => connectServerHostFromSubsonic(activeSession()?.normalizedServerUrl ?? null));
  const displayedConnectServerHost = createMemo(() => (useSubsonicServerHost() ? subsonicConnectServerHost() : connectServerHost()));
  const derivedConnectServerUrl = createMemo(() => {
    try {
      return resolveConnectServerUrl(
        {
          ...settingsStore.connect,
          enabled: connectEnabled(),
          useSubsonicServerHost: useSubsonicServerHost(),
          connectServerPort: normalizePort(connectServerPort()),
          connectServerHost: connectServerHost().trim() || null,
          deviceName: connectDeviceName().trim() || null,
          allowInsecureConnectServer: allowInsecureConnectServer(),
        },
        activeSession()?.normalizedServerUrl ?? null
      );
    } catch (error) {
      return error instanceof Error ? error.message : String(error);
    }
  });
  const connectRuntimeRestarting = createMemo(() => connectStore.runtime.message === 'connect: restarting');
  const connectButtonLabel = createMemo(() => (connectStore.runtime.connected || connectRuntimeRestarting() ? 'Update' : 'Connect'));
  const coverArtCacheSummary = createMemo(() => {
    const status = coverArtCacheStatus();
    const entryCount = status?.entryCount ?? 0;
    const totalBytes = status?.totalBytes ?? 0;

    return `${entryCount.toLocaleString()} Arts, ${formatBytes(totalBytes)}`;
  });

  const refreshCoverArtCacheStatus = async () => {
    setCoverArtCacheMessage(null);
    try {
      const result = await commands.getCoverArtCacheStatus();
      if (result.status === 'error') {
        setCoverArtCacheMessage(result.error);
        return;
      }

      setCoverArtCacheStatus(result.data);
    } catch (error) {
      setCoverArtCacheMessage(error instanceof Error ? error.message : String(error));
    }
  };

  onMount(() => {
    if (connectDeviceName().trim().length > 0) {
      return;
    }

    void commands.getDefaultDeviceName().then((name) => {
      if (connectDeviceName().trim().length === 0 && name.trim().length > 0) {
        setConnectDeviceName(name);
      }
    });
  });

  const clearCoverArtCache = async () => {
    setCoverArtCacheBusy(true);
    setCoverArtCacheMessage(null);
    try {
      const result = await commands.clearCoverArtCache();
      if (result.status === 'error') {
        setCoverArtCacheMessage(result.error);
        return;
      }

      setCoverArtCacheStatus(result.data);
    } catch (error) {
      setCoverArtCacheMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setCoverArtCacheBusy(false);
    }
  };

  const applyConnectSettings = async () => {
    setConnectActionBusy(true);
    setConnectActionError(null);
    try {
      const result = await setConnectSettings({
        ...settingsStore.connect,
        enabled: connectEnabled(),
        useSubsonicServerHost: useSubsonicServerHost(),
        connectServerPort: normalizePort(connectServerPort()),
        connectServerHost: connectServerHost().trim() || null,
        deviceName: connectDeviceName().trim() || null,
        allowInsecureConnectServer: allowInsecureConnectServer(),
      });
      if (result.status === 'error') {
        setConnectActionError(result.error);
        return;
      }

      await refreshConnectServerStatus();
      await refreshConnectRuntimeStatus();
    } catch (error) {
      setConnectActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setConnectActionBusy(false);
    }
  };

  const exportBackup = async () => {
    setBackupBusy(true);
    setBackupMessage(null);
    try {
      const result = await commands.exportServerBackup();
      if (result.status === 'error') {
        setBackupMessage(result.error);
        return;
      }
      setBackupMessage(`Exported ${result.data.profileCount} profile(s): ${result.data.path}`);
    } catch (error) {
      setBackupMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBackupBusy(false);
    }
  };

  const importBackup = async () => {
    setBackupBusy(true);
    setBackupMessage(null);
    try {
      const result = await commands.importServerBackup({ replaceExisting: true });
      if (result.status === 'error') {
        setBackupMessage(result.error);
        return;
      }
      const bootstrapResult = await commands.bootstrapAppState();
      if (bootstrapResult.status === 'error') {
        setBackupMessage(bootstrapResult.error);
        return;
      }
      await loadBootstrapToStore(bootstrapResult.data);
      syncConnectForm();
      setBackupMessage(`Imported ${result.data.profileCount} profile(s): ${result.data.path}`);
    } catch (error) {
      setBackupMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBackupBusy(false);
    }
  };

  createEffect(() => {
    settingsStore.connect.enabled;
    settingsStore.connect.useSubsonicServerHost;
    settingsStore.connect.connectServerPort;
    settingsStore.connect.connectServerHost;
    settingsStore.connect.allowInsecureConnectServer;
    sessionStore.activeSession?.normalizedServerUrl;
    void refreshConnectServerStatus();
  });

  createEffect(() => {
    sessionStore.activeSession?.profileId;
    setCoverArtCacheMessage(null);
    void refreshCoverArtCacheStatus();
  });

  createEffect(() => {
    const intervalId = window.setInterval(() => {
      void refreshConnectRuntimeStatus();
    }, 3000);
    void refreshConnectRuntimeStatus();
    onCleanup(() => window.clearInterval(intervalId));
  });

  createEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void startConnectDevicesSync().then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
        return;
      }
      unlisten = nextUnlisten;
    });
    onCleanup(() => {
      disposed = true;
      unlisten?.();
    });
  });

  return (
    <div class='home-surface-root gap-4 p-4 lg:p-5'>
      <Heading1>Settings</Heading1>

      <section class='bg-primary-plane border-primary-border flex flex-col gap-3 rounded-lg border p-4'>
        <div class='flex items-center justify-between gap-3'>
          <Heading2>Appearance</Heading2>
        </div>
        <div class='flex flex-wrap gap-2'>
          <For each={themeOptions}>
            {(option) => (
              <button
                classList={{
                  'bg-primary-selected border-accent text-accent': currentTheme() === option.value,
                }}
                onClick={() => applyTheme(option.value)}
              >
                {option.label}
              </button>
            )}
          </For>
        </div>
      </section>

      <Show when={supportsGaplessPlayback()}>
        <section class='bg-primary-plane border-primary-border flex flex-col gap-4 rounded-lg border p-4'>
          <Heading2>Playback Defaults</Heading2>

          <label class='flex items-center justify-between gap-4'>
            <span>Gapless Playback</span>
            <input
              type='checkbox'
              checked={isGaplessEnabled()}
              class='accent-accent h-4 w-4 border border-current'
              onChange={(event) => setPlaybackSetting('gaplessPlaybackEnabled', event.currentTarget.checked)}
            />
          </label>
        </section>
      </Show>

      <section class='bg-primary-plane border-primary-border flex flex-col gap-3 rounded-lg border p-4'>
        <Heading2>Server</Heading2>
        <Show when={activeSession()} fallback={<p class='text-secondary-text leading-5'>No active server session.</p>}>
          {(session) => (
            <div class='flex flex-col gap-2'>
              <p>{session().username}</p>
              <p class='text-secondary-text leading-5'>{session().normalizedServerUrl}</p>
              <p class='text-secondary-text text-xs'>
                API {session().apiVersion}
                <Show when={session().serverType}>{(serverType) => <> · {serverType()}</>}</Show>
                <Show when={session().serverVersion}>{(serverVersion) => <> {serverVersion()}</>}</Show>
              </p>
            </div>
          )}
        </Show>
        <div class='flex flex-wrap gap-2'>
          <button type='button' disabled={backupBusy()} onClick={exportBackup}>
            Export Backup
          </button>
          <button type='button' disabled={backupBusy()} onClick={importBackup}>
            Import Backup
          </button>
        </div>
        <p class='text-secondary-text text-xs leading-5'>Backup includes server credentials. Keep the exported JSON private.</p>
        <Show when={backupMessage()}>{(message) => <p class='text-secondary-text text-xs leading-5'>{message()}</p>}</Show>
      </section>

      <section class='bg-primary-plane border-primary-border flex flex-col gap-3 rounded-lg border p-4'>
        <Heading2>Album Art Cache</Heading2>
        <p class='text-secondary-text text-xs leading-5'>{coverArtCacheSummary()}</p>
        <div class='flex w-full justify-end'>
          <button type='button' disabled={coverArtCacheBusy() || !activeSession()} onClick={() => void clearCoverArtCache()}>
            Clear Cache
          </button>
        </div>
        <Show when={coverArtCacheMessage()}>{(message) => <p class='text-secondary-text text-xs leading-5'>{message()}</p>}</Show>
      </section>

      <section class='bg-primary-plane border-primary-border flex flex-col gap-4 rounded-lg border p-4'>
        <Heading2>Connect</Heading2>

        <label class='flex items-center justify-between gap-4'>
          <span>Use Transonic Connect</span>
          <input
            type='checkbox'
            checked={connectEnabled()}
            class='accent-accent h-4 w-4 border border-current'
            onChange={(event) => setConnectEnabled(event.currentTarget.checked)}
          />
        </label>

        <fieldset disabled={!connectEnabled()} classList={{ 'opacity-50': !connectEnabled() }} class='flex flex-col gap-4'>
          <label class='flex items-center justify-between gap-4'>
            <span>Use Subsonic Server IP</span>
            <input
              type='checkbox'
              checked={useSubsonicServerHost()}
              class='accent-accent h-4 w-4 border border-current'
              onChange={(event) => {
                const checked = event.currentTarget.checked;
                setUseSubsonicServerHost(checked);
                if (!checked && connectServerHost().trim().length === 0) {
                  setConnectServerHost(subsonicConnectServerHost());
                }
              }}
            />
          </label>

          <div class='flex flex-col gap-2'>
            <span>Connect Server URL</span>
            <div class='grid grid-cols-[minmax(0,1fr)_7rem] gap-2'>
              <input
                value={displayedConnectServerHost()}
                disabled={useSubsonicServerHost()}
                placeholder='https://connect.example'
                class='bg-primary-surface border-primary-border rounded border px-3 py-2'
                classList={{
                  'text-secondary-text cursor-not-allowed opacity-50': useSubsonicServerHost(),
                }}
                onInput={(event) => {
                  const split = splitUrlHostAndPort(event.currentTarget.value);
                  setConnectServerHost(split.host);
                  if (split.port !== null) {
                    setConnectServerPort(String(split.port));
                  }
                }}
              />
              <input
                type='number'
                min='1'
                max='65535'
                value={connectServerPort()}
                class='bg-primary-surface border-primary-border rounded border px-3 py-2'
                onInput={(event) => setConnectServerPort(event.currentTarget.value)}
              />
            </div>
          </div>

          <label class='flex flex-col gap-2'>
            <span>Device Name</span>
            <input
              value={connectDeviceName()}
              placeholder='This device'
              class='bg-primary-surface border-primary-border rounded border px-3 py-2'
              onInput={(event) => setConnectDeviceName(event.currentTarget.value)}
            />
          </label>

          <label class='flex items-center justify-between gap-4'>
            <span>Allow LAN HTTP Connect Server</span>
            <input
              type='checkbox'
              checked={allowInsecureConnectServer()}
              class='accent-accent h-4 w-4 border border-current'
              onChange={(event) => setAllowInsecureConnectServer(event.currentTarget.checked)}
            />
          </label>

          <div class='flex items-center gap-3'>
            <button type='button' disabled={connectActionBusy() || connectRuntimeRestarting()} onClick={applyConnectSettings}>
              {connectButtonLabel()}
            </button>
            <Show when={connectActionError()}>{(message) => <p class='text-secondary-text text-xs leading-5'>{message()}</p>}</Show>
          </div>

          <div class='text-secondary-text flex flex-col gap-1 text-xs leading-5'>
            <p>Target: {derivedConnectServerUrl() ?? 'disabled'}</p>
            <p>Health: {connectStore.status}</p>
            <p>
              Runtime: {connectStore.runtime.connected ? 'connected' : connectStore.runtime.enabled ? 'disconnected' : 'disabled'} · seq{' '}
              {connectStore.runtime.seq}
            </p>
            <Show when={connectStore.version}>
              {(version) => (
                <p>
                  Server {version()} · protocol {connectStore.protocolVersion}
                </p>
              )}
            </Show>
            <Show when={connectStore.message}>{(message) => <p>{message()}</p>}</Show>
            <Show when={connectStore.runtime.message}>{(message) => <p>{message()}</p>}</Show>
          </div>

          <p class='text-secondary-text text-xs leading-5'>Device ID: {settingsStore.connect.deviceId}</p>

          <Show when={connectStore.devices.length > 0}>
            <div class='flex flex-col gap-2'>
              <For each={connectStore.devices}>
                {(entry) => (
                  <div class='border-primary-border bg-primary-surface flex flex-col gap-1 rounded border p-3'>
                    <div class='flex items-center justify-between gap-3'>
                      <p class='font-medium'>{entry.device.displayName || entry.device.deviceId}</p>
                      <span class={entry.device.online ? 'text-accent' : 'text-secondary-text'}>{entry.device.online ? 'online' : 'offline'}</span>
                    </div>
                    <p class='text-secondary-text text-xs'>
                      {entry.device.platform || 'unknown'} · {entry.device.deviceId}
                    </p>
                    <Show when={entry.playback} fallback={<p class='text-secondary-text text-xs'>No playback state</p>}>
                      {(playback) => (
                        <p class='text-sm'>
                          {playback().state.playingState} · {currentSongLabel(playback())} · {formatPosition(playback().positionMs)}
                        </p>
                      )}
                    </Show>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </fieldset>
      </section>
    </div>
  );
}

export default SettingsRoute;
