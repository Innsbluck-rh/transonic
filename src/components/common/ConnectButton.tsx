import { Component, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { ConnectDevicePresence } from '~/bindings';
import { resolveCurrentPlaybackDevice } from '~/features/connect/playbackDevice';
import { connectStore } from '~/stores/ConnectStore';
import ConnectDeviceCard from '../settings/ConnectDeviceCard';
import Heading3 from './text/Heading3';

const ConnectButton: Component = () => {
  const activePlaybackDevice = (): ConnectDevicePresence | undefined =>
    resolveCurrentPlaybackDevice({
      connected: connectStore.runtime.connected,
      ownDeviceId: connectStore.runtime.deviceId,
      devices: connectStore.devices,
      sharedPlayback: connectStore.sharedPlayback,
    });

  const activeDeviceId = () => (connectStore.runtime.connected ? (connectStore.sharedPlayback?.activeDeviceId ?? null) : null);
  const isOwnActiveDevice = () => {
    const ownDeviceId = connectStore.runtime.deviceId;
    return ownDeviceId !== null && activeDeviceId() === ownDeviceId;
  };
  const hasActiveDevice = () => isOwnActiveDevice() || activePlaybackDevice() !== undefined;
  const isActiveDevicePlaying = () => hasActiveDevice() && connectStore.sharedPlayback?.state.playingState === 'playing';
  const activeDeviceLabel = () => {
    if (isOwnActiveDevice()) {
      return 'This Device';
    }
    return activePlaybackDevice()?.displayName || '(Nothing Active)';
  };

  let buttonEl: HTMLButtonElement | undefined;
  let menuEl: HTMLDivElement | undefined;
  const [menuOpened, setMenuOpened] = createSignal(false);

  const onOutsideMenuClick = (e: MouseEvent) => {
    if (!menuOpened()) {
      return;
    }

    const target = e.target;
    if (!(target instanceof Node)) {
      return;
    }

    if (!menuEl?.contains(target) && !buttonEl?.contains(target)) {
      setMenuOpened(false);
    }
  };

  onMount(() => {
    document.addEventListener('click', onOutsideMenuClick);
  });

  onCleanup(() => {
    document.removeEventListener('click', onOutsideMenuClick);
  });

  return (
    <div class='relative flex overflow-visible'>
      <div class='relative'>
        <button
          ref={(el) => (buttonEl = el)}
          type='button'
          aria-expanded={menuOpened()}
          aria-haspopup='menu'
          data-tauri-drag-exclude
          class='bg-primary-surface hover:bg-primary-hover border-secondary-border sp:gap-2.5 sp:px-3 sp:py-2 flex cursor-pointer flex-row items-center gap-2 rounded-full border px-2.5 py-1.5'
          onClick={() => setMenuOpened((opened) => !opened)}
        >
          <i
            class={`${isActiveDevicePlaying() ? 'icon-[material-symbols--volume-up]' : 'icon-[material-symbols--devices]'} sp:-m-[1.75px] sp:text-[17.5px] text-sm`}
            classList={{
              'text-accent': isActiveDevicePlaying(),
              'text-primary-text': !isActiveDevicePlaying() && hasActiveDevice(),
              'text-secondary-text': !isActiveDevicePlaying() && !hasActiveDevice(),
            }}
          />

          <p
            class='sp:text-sm text-xs leading-none font-bold'
            classList={{
              italic: isOwnActiveDevice() || !hasActiveDevice(),
              'text-accent': isActiveDevicePlaying(),
              'text-primary-text': !isActiveDevicePlaying() && hasActiveDevice(),
              'text-secondary-text': !isActiveDevicePlaying() && !hasActiveDevice(),
            }}
          >
            {activeDeviceLabel()}
          </p>
        </button>
      </div>
      <Show when={menuOpened()}>
        <div
          ref={(el) => (menuEl = el)}
          role='menu'
          data-tauri-drag-exclude
          class='border-secondary-border bg-primary-surface absolute right-0 -bottom-3 z-50 flex w-max min-w-56 translate-y-full flex-col rounded border shadow-[0px_3px_8px_rgba(0,0,0,0.25)]'
        >
          <div class='border-secondary-border flex w-full flex-row items-center border-b px-2 py-1'>
            <Heading3 class='text-secondary-text'>Devices</Heading3>
          </div>
          <Show when={connectStore.devices.length > 0} fallback={<p class='self-center p-3'>there's no devices</p>}>
            <div class='flex min-w-0 flex-1 flex-col gap-1 p-0.5'>
              <For each={connectStore.devices}>{(device) => <ConnectDeviceCard device={device} />}</For>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default ConnectButton;
