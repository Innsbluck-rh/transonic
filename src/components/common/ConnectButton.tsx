import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { ConnectDevicePresence } from '~/bindings';
import { resolveExternalPlaybackDevice } from '~/features/connect/playbackDevice';
import { connectStore } from '~/stores/ConnectStore';
import ConnectDeviceCard from '../settings/ConnectDeviceCard';
import Heading3 from './Heading3';

const ConnectButton: Component = () => {
  const externalPlaybackDevice = createMemo<ConnectDevicePresence | undefined>(() => {
    return resolveExternalPlaybackDevice({
      connected: connectStore.runtime.connected,
      ownDeviceId: connectStore.runtime.deviceId,
      devices: connectStore.devices,
      sharedPlayback: connectStore.sharedPlayback,
    });
  });

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
      <div class='relative overflow-visible'>
        <button
          ref={(el) => (buttonEl = el)}
          type='button'
          aria-expanded={menuOpened()}
          aria-haspopup='menu'
          data-tauri-drag-exclude
          class='bg-primary-plane hover:bg-primary-hover flex cursor-pointer flex-row items-center gap-3 overflow-visible rounded-full border-0 p-1.5 lg:gap-2'
          onClick={() => setMenuOpened((opened) => !opened)}
        >
          <Icon
            class='scale-150 lg:scale-100'
            icon='material-symbols:devices'
            classList={{
              'text-primary-text': !menuOpened(),
              'text-accent': menuOpened(),
            }}
          />

          <Show when={externalPlaybackDevice()}>
            {(device) => (
              <p
                class='text-sm leading-none font-bold lg:text-xs'
                classList={{
                  'text-primary-text': !menuOpened(),
                  'text-accent': menuOpened(),
                }}
              >
                {device().displayName}
              </p>
            )}
          </Show>
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
