import { Icon } from '@iconify-icon/solid';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Component, onCleanup, onMount } from 'solid-js';
import { createStore } from 'solid-js/store';

const WindowButtonSection: Component = () => {
  const [windowStore, setWindowStore] = createStore({
    minimizable: false,
    minimized: false,
    maximizable: false,
    maximized: false,
    closable: false,
  });

  async function updateWindowStore() {
    const window = getCurrentWindow();
    setWindowStore({
      minimizable: await window.isMinimizable(),
      minimized: await window.isMinimized(),
      maximizable: await window.isMaximizable(),
      maximized: await window.isMaximized(),
      closable: await window.isClosable(),
    });
  }

  onMount(async () => {
    updateWindowStore();

    const appWindow = getCurrentWindow();
    const unlisten = await appWindow.onResized(({ payload: size }) => {
      updateWindowStore();
    });

    onCleanup(() => {
      unlisten();
    });
  });

  return (
    <div class='flex h-full'>
      <div
        data-tauri-drag-exclude
        class='hover:bg-primary-hover ml-2 flex h-full w-11 items-center justify-center'
        classList={{
          ['cursor-pointer']: windowStore.minimizable,
        }}
        onClick={() => getCurrentWindow().minimize()}
      >
        <Icon class='text-primary-text text-md' icon='material-symbols:minimize' />
      </div>
      <div
        data-tauri-drag-exclude
        class='hover:bg-primary-hover flex h-full w-11 items-center justify-center'
        classList={{
          ['cursor-pointer']: windowStore.maximizable,
        }}
        onClick={() => getCurrentWindow().toggleMaximize()}
      >
        <Icon class='text-primary-text text-sm' icon={windowStore.maximized ? 'material-symbols:stack-outline' : 'material-symbols:square-outline'} />
      </div>
      <div
        data-tauri-drag-exclude
        class='group hover:bg-primary-hover flex h-full w-11 items-center justify-center'
        classList={{
          ['cursor-pointer']: windowStore.closable,
        }}
        onClick={() => getCurrentWindow().close()}
      >
        <Icon class='text-primary-text text-md group-hover:text-red-500' icon='material-symbols:close' />
      </div>
    </div>
  );
};

export default WindowButtonSection;
