import { Icon } from '@iconify-icon/solid';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Component, onCleanup, onMount, Show } from 'solid-js';
import { createStore } from 'solid-js/store';
import { resolveSettingsRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { isSP } from '~/utils/isSP';
import Title from '../Title';
import ThemeToggle from './ThemeToggle';

interface HeaderProps {
  title?: string;
  titleHref?: string;
  shouldShowProfiles?: boolean;
}

const Header: Component<HeaderProps> = (props) => {
  const navigate = useSPNavigate();

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
    console.log(windowStore);
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
    <div class='bg-primary-plane border-primary-border flex h-12 flex-row items-center border-b px-1.5' data-tauri-drag-region>
      <Show when={props.titleHref} fallback={<Title>{props.title ?? 'Transonic'}</Title>}>
        <Title class='cursor-pointer' onClick={() => navigate(props.titleHref!)}>
          {props.title ?? 'Transonic'}
        </Title>
      </Show>
      <div class='flex-1' />
      <div class='flex flex-row gap-0'>
        <ThemeToggle />
        <div class='relative overflow-visible'>
          <div
            class='bg-primary-plane hover:bg-primary-hover flex cursor-pointer flex-row items-center gap-2 rounded-full p-2'
            onClick={() => navigate(resolveSettingsRoute())}
          >
            <Icon class='text-primary-text text-[12px]' icon='pixelarticons:settings-2-sharp' />
          </div>
        </div>
      </div>

      <Show when={!isSP()}>
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
          <Icon
            class='text-primary-text text-sm'
            icon={windowStore.maximized ? 'material-symbols:stack-outline' : 'material-symbols:square-outline'}
          />
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
      </Show>
    </div>
  );
};

export default Header;
