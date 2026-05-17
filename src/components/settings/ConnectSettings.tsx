import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { commands } from '~/bindings';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
import {
  connectServerHostFromSubsonic,
  refreshConnectRuntimeStatus,
  refreshConnectServerStatus,
  splitUrlHostAndPort,
  startConnectDevicesSync,
} from '~/features/connect/service';
import { setConnectSettings, settingsStore } from '~/features/settings/service';
import { connectStore } from '~/stores/ConnectStore';
import { sessionStore } from '~/stores/SessionStore';
import ConnectDeviceCard, { currentSong } from './ConnectDeviceCard';
import SettingSection from './SettingSection';

function connectStatusLabel() {
  if (!settingsStore.connect.enabled) {
    return 'Off';
  }
  if (connectStore.runtime.connected) {
    return 'Connected';
  }
  if (connectStore.status === 'checking') {
    return 'Checking';
  }
  if (connectStore.status === 'available') {
    return 'Ready';
  }
  return 'Unavailable';
}

function connectStatusClass() {
  if (connectStore.runtime.connected) {
    return 'text-accent';
  }
  if (connectStore.status === 'available' || connectStore.status === 'checking') {
    return 'text-secondary-text';
  }
  return 'text-red-500';
}

function normalizePort(value: string) {
  const port = Number(value);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    return 4747;
  }
  return port;
}

function ConnectSettings() {
  const activeSession = () => sessionStore.activeSession;
  const [connectEnabled, setConnectEnabled] = createSignal(settingsStore.connect.enabled === true);
  const [useSubsonicServerHost, setUseSubsonicServerHost] = createSignal(settingsStore.connect.useSubsonicServerHost !== false);
  const [connectServerPort, setConnectServerPort] = createSignal(String(settingsStore.connect.connectServerPort || 4747));
  const [connectServerHost, setConnectServerHost] = createSignal(settingsStore.connect.connectServerHost ?? '');
  const [connectDeviceName, setConnectDeviceName] = createSignal(settingsStore.connect.deviceName ?? '');
  const [allowInsecureConnectServer, setAllowInsecureConnectServer] = createSignal(settingsStore.connect.allowInsecureConnectServer === true);
  const [connectActionError, setConnectActionError] = createSignal<string | null>(null);
  const [connectActionBusy, setConnectActionBusy] = createSignal(false);
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
  const connectRuntimeRestarting = createMemo(() => connectStore.runtime.message === 'connect: restarting');
  const connectButtonLabel = createMemo(() => (connectStore.runtime.connected || connectRuntimeRestarting() ? 'Update' : 'Connect'));
  const connectCoverArtMap = useCoverArtMap(
    () =>
      connectStore.devices
        .map((entry) => (entry.playback ? currentSong(entry.playback)?.coverArtId : null))
        .filter((coverArtId): coverArtId is string => typeof coverArtId === 'string' && coverArtId.length > 0),
    CoverArtSizes.sm,
    { cachedFallbackSizes: [CoverArtSizes.lg, CoverArtSizes.md] }
  );

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

  createEffect(() => {
    settingsStore.connect.enabled;
    settingsStore.connect.useSubsonicServerHost;
    settingsStore.connect.connectServerPort;
    settingsStore.connect.connectServerHost;
    settingsStore.connect.deviceName;
    settingsStore.connect.allowInsecureConnectServer;
    syncConnectForm();
  });

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
    <SettingSection title='Connect'>
      <label class='flex min-w-0 items-center justify-between gap-4'>
        <span class='min-w-0'>Use Transonic Connect</span>
        <input
          type='checkbox'
          checked={connectEnabled()}
          class='accent-accent h-4 w-4 border border-current'
          onChange={(event) => setConnectEnabled(event.currentTarget.checked)}
        />
      </label>

      <fieldset disabled={!connectEnabled()} classList={{ 'opacity-50': !connectEnabled() }} class='flex min-w-0 flex-col gap-4'>
        <label class='flex min-w-0 items-center justify-between gap-4'>
          <span class='min-w-0'>Use Subsonic Host URL</span>
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

        <div class='flex min-w-0 flex-col gap-2'>
          <span>Connect Server URL / Port</span>
          <div class='grid min-w-0 grid-cols-[minmax(0,1fr)_7rem] gap-2'>
            <input
              value={displayedConnectServerHost()}
              disabled={useSubsonicServerHost()}
              placeholder='https://connect.example'
              class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
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
              class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
              onInput={(event) => setConnectServerPort(event.currentTarget.value)}
            />
          </div>
        </div>

        <label class='flex min-w-0 flex-col gap-2'>
          <span>Device Name</span>
          <input
            value={connectDeviceName()}
            placeholder='This device'
            class='bg-primary-surface border-primary-border min-w-0 rounded border px-3 py-2'
            onInput={(event) => setConnectDeviceName(event.currentTarget.value)}
          />
        </label>

        <label class='flex min-w-0 items-center justify-between gap-4'>
          <span class='min-w-0'>Allow LAN HTTP Connect Server</span>
          <input
            type='checkbox'
            checked={allowInsecureConnectServer()}
            class='accent-accent h-4 w-4 border border-current'
            onChange={(event) => setAllowInsecureConnectServer(event.currentTarget.checked)}
          />
        </label>

        <div class='flex min-w-0 flex-wrap items-center gap-3'>
          <button type='button' disabled={connectActionBusy() || connectRuntimeRestarting()} onClick={applyConnectSettings}>
            {connectButtonLabel()}
          </button>
          <Show when={connectActionError()}>{(message) => <p class='text-secondary-text min-w-0 text-xs leading-5 break-words'>{message()}</p>}</Show>
        </div>

        <div class='flex min-w-0 flex-wrap items-center gap-2 text-xs leading-5'>
          <span class={`${connectStatusClass()} shrink-0`}>{connectStatusLabel()}</span>
          <Show when={connectStore.version}>{(version) => <span class='text-secondary-text break-words'>Server {version()}</span>}</Show>
          <Show when={connectStore.message}>{(message) => <span class='text-secondary-text min-w-0 break-words'>{message()}</span>}</Show>
          <Show when={connectStore.runtime.message && !connectStore.runtime.connected}>
            {(message) => <span class='text-secondary-text min-w-0 break-words'>{message()}</span>}
          </Show>
        </div>

        <Show when={connectStore.devices.length > 0}>
          <div class='flex min-w-0 flex-col gap-2'>
            <For each={connectStore.devices}>
              {(entry) => <ConnectDeviceCard entry={entry} coverArtSrc={(coverArtId) => connectCoverArtMap.src(coverArtId ?? undefined)} />}
            </For>
          </div>
        </Show>
      </fieldset>
    </SettingSection>
  );
}

export default ConnectSettings;
