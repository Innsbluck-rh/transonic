import { Icon } from '@iconify-icon/solid';
import { Component, Show } from 'solid-js';
import { resolveSettingsRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { sessionStore } from '~/stores/SessionStore';
import Title from '../Title';
import ServerProfile from './ServerProfile';
import ThemeToggle from './ThemeToggle';

interface HeaderProps {
  title?: string;
  titleHref?: string;
  shouldShowProfiles?: boolean;
}

const Header: Component<HeaderProps> = (props) => {
  const navigate = useSPNavigate();

  return (
    <div class='bg-primary-plane border-primary-border flex h-12 flex-row items-center border-b px-2.5'>
      <Show when={props.titleHref} fallback={<Title>{props.title ?? 'Transonic'}</Title>}>
        <Title class='cursor-pointer' onClick={() => navigate(props.titleHref!)}>
          {props.title ?? 'Transonic'}
        </Title>
      </Show>
      <div class='flex-1' />
      <div class='flex flex-row gap-1'>
        <div class='relative overflow-visible'>
          <div
            class='bg-primary-plane hover:bg-primary-hover flex cursor-pointer flex-row items-center gap-2 rounded-full p-2'
            onClick={() => navigate(resolveSettingsRoute())}
          >
            <Icon class='text-primary-text text-[16px]' icon='pixelarticons:settings-2-sharp' />
          </div>
        </div>
        <ThemeToggle />
        {/* <input class='text-xs mr-2' placeholder='search...' /> */}
        <Show when={props.shouldShowProfiles}>
          <ServerProfile profiles={sessionStore.profiles} />
        </Show>
      </div>
    </div>
  );
};

export default Header;
