import { Icon } from '@iconify-icon/solid';
import { useLocation } from '@solidjs/router';
import { Component, Show } from 'solid-js';
import { resolveSearchRoute, resolveSettingsRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { settingsStore } from '~/features/settings/service';
import ConnectButton from '../ConnectButton';
import Title from '../text/Title';
import WindowButtonSection from './WindowButtonSection';

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
      <div class='ml-2 flex flex-row items-center gap-0'>
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

      <WindowButtonSection />
    </div>
  );
};

export default Header;
