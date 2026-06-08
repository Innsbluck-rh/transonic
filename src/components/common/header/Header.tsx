import { Icon } from '@iconify-icon/solid';
import { useLocation } from '@solidjs/router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Component, onCleanup, onMount, Show } from 'solid-js';
import { createStore } from 'solid-js/store';
import { resolveSearchRoute, resolveSettingsRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { settingsStore } from '~/features/settings/service';
import ConnectButton from '../ConnectButton';
import { openPlaybackStreamInfoDialog } from '../dialog/PlaybackStreamInfoDialog';
import Title from '../Title';

interface HeaderProps {
  title?: string;
  titleHref?: string;
  shouldShowProfiles?: boolean;
}

const Header: Component<HeaderProps> = (props) => {
  const location = useLocation();
  const isInSettingPage = () => location.pathname.startsWith('/settings') || location.pathname.startsWith('/sp/settings');
  const isInSearchPage = () => location.pathname.startsWith('/search') || location.pathname.startsWith('/sp/search');
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
    <div class='bg-primary-plane border-primary-border flex h-10 max-h-10 min-h-10 flex-row items-center border-b px-1.5' data-tauri-drag-region>
      <Show when={props.titleHref} fallback={<Title>{props.title ?? 'Transonic'}</Title>}>
        <Title class='cursor-pointer' onClick={() => navigate(props.titleHref!)}>
          {props.title ?? 'Transonic'}
        </Title>
      </Show>
      <div class='flex-1' />

      <Show when={settingsStore.connect.enabled}>
        <ConnectButton />
      </Show>
      <button
        data-tauri-drag-exclude
        class='bg-primary-plane hover:bg-primary-hover ml-2 flex cursor-pointer flex-row items-center rounded-full p-1.5'
        type='button'
        title='Playback stream info'
        aria-label='Playback stream info'
        onClick={() => openPlaybackStreamInfoDialog()}
      >
        <Icon class='text-primary-text' icon='material-symbols:info-outline' />
      </button>
      <div class='ml-2 flex flex-row gap-0'>
        <div
          class='bg-primary-plane hover:bg-primary-hover flex cursor-pointer flex-row items-center rounded-full p-1.5'
          onClick={() => navigate(resolveSearchRoute())}
        >
          <Icon
            classList={{
              'text-accent': isInSearchPage(),
              'text-primary-text': !isInSearchPage(),
            }}
            icon='material-symbols:search'
          />
        </div>
      </div>
      <div class='ml-2 flex flex-row gap-0'>
        <div
          class='bg-primary-plane hover:bg-primary-hover flex cursor-pointer flex-row items-center rounded-full p-1.5'
          onClick={() => navigate(resolveSettingsRoute())}
        >
          <Icon
            classList={{
              'text-accent': isInSettingPage(),
              'text-primary-text': !isInSettingPage(),
            }}
            icon='material-symbols:settings'
          />
        </div>
      </div>

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

export default Header;
