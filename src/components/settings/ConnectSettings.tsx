import { createEffect, createMemo, createSignal, onMount, Show } from 'solid-js';
import { commands } from '~/bindings';
import {
  connectServerHostFromSubsonic,
  refreshConnectRuntimeStatus,
  refreshConnectServerStatus,
  splitUrlHostAndPort,
} from '~/features/connect/service';
import { setConnectSettings, settingsStore } from '~/features/settings/service';
import { connectStore } from '~/stores/ConnectStore';
import { sessionStore } from '~/stores/SessionStore';
import SettingSection from './SettingSection';

function optionalTrimmedText(value: string) {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

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
  const connectSettingsDraft = createMemo(() => ({
    useSubsonicServerHost: useSubsonicServerHost(),
    connectServerPort: normalizePort(connectServerPort()),
    connectServerHost: optionalTrimmedText(connectServerHost()),
    deviceName: optionalTrimmedText(connectDeviceName()),
    allowInsecureConnectServer: allowInsecureConnectServer(),
  }));
  const advancedSettingsDirty = createMemo(() => {
    const draft = connectSettingsDraft();
    return (
      (settingsStore.connect.useSubsonicServerHost !== false) !== draft.useSubsonicServerHost ||
      connectServerPort().trim() !== String(settingsStore.connect.connectServerPort || 4747) ||
      (settingsStore.connect.connectServerHost ?? null) !== draft.connectServerHost ||
      (settingsStore.connect.deviceName ?? null) !== draft.deviceName ||
      (settingsStore.connect.allowInsecureConnectServer === true) !== draft.allowInsecureConnectServer
    );
  });

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

  const applyConnectSettings = async (enabled = connectEnabled()) => {
    setConnectActionBusy(true);
    setConnectActionError(null);
    try {
      const draft = connectSettingsDraft();
      const result = await setConnectSettings({
        ...settingsStore.connect,
        enabled,
        ...draft,
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

  return (
    <SettingSection title='Connect'>
      <label class='flex min-w-0 items-center justify-between gap-4'>
        <span class='min-w-0'>Use Transonic Connect</span>
        <input
          type='checkbox'
          checked={connectEnabled()}
          disabled={connectActionBusy()}
          onChange={(event) => {
            const enabled = event.currentTarget.checked;
            setConnectEnabled(enabled);
            void applyConnectSettings(enabled);
          }}
        />
      </label>

      <Show when={connectActionError()}>{(message) => <p class='text-secondary-text min-w-0 text-xs leading-5 break-words'>{message()}</p>}</Show>

      <Show when={connectEnabled()}>
        <fieldset class='flex min-w-0 flex-col gap-4'>
          <details class='flex min-w-0 flex-col gap-3'>
            <summary class='text-secondary-text cursor-pointer text-sm leading-5 font-bold'>Advanced</summary>
            <div class='mt-3 flex min-w-0 flex-col gap-4'>
              <label class='flex min-w-0 items-center justify-between gap-4'>
                <span class='min-w-0'>Use Subsonic Host URL</span>
                <input
                  type='checkbox'
                  checked={useSubsonicServerHost()}
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
                    onInput={(event) => setConnectServerPort(event.currentTarget.value)}
                  />
                </div>
              </div>

              <label class='flex min-w-0 flex-col gap-2'>
                <span>Device Name</span>
                <input value={connectDeviceName()} placeholder='This device' onInput={(event) => setConnectDeviceName(event.currentTarget.value)} />
              </label>

              <label class='flex min-w-0 items-center justify-between gap-4'>
                <span class='min-w-0'>Allow LAN HTTP Connect Server</span>
                <input
                  type='checkbox'
                  checked={allowInsecureConnectServer()}
                  onChange={(event) => setAllowInsecureConnectServer(event.currentTarget.checked)}
                />
              </label>

              <div class='flex min-w-0 flex-wrap items-center justify-end gap-3'>
                <button
                  type='button'
                  disabled={!connectEnabled() || !advancedSettingsDirty() || connectActionBusy() || connectRuntimeRestarting()}
                  onClick={() => void applyConnectSettings(true)}
                >
                  Apply
                </button>
              </div>
            </div>
          </details>

          {/*<Show when={connectStore.devices.length > 0}>
            <div class='flex min-w-0 flex-col gap-1'>
              <For each={connectStore.devices}>{(device) => <ConnectDeviceCard device={device} />}</For>
            </div>
          </Show>*/}
        </fieldset>

        <div class='flex min-w-0 flex-wrap items-center gap-2 text-xs leading-5'>
          <span class={`${connectStatusClass()} shrink-0`}>{connectStatusLabel()}</span>
          <Show when={connectStore.version}>{(version) => <span class='text-secondary-text break-words'>Server {version()}</span>}</Show>
          <Show when={connectStore.message}>{(message) => <span class='text-secondary-text min-w-0 break-words'>{message()}</span>}</Show>
          <Show when={connectStore.runtime.message && !connectStore.runtime.connected}>
            {(message) => <span class='text-secondary-text min-w-0 break-words'>{message()}</span>}
          </Show>
        </div>
      </Show>
    </SettingSection>
  );
}

export default ConnectSettings;
