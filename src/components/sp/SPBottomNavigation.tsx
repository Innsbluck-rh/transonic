import { Icon } from '@iconify-icon/solid';
import { useLocation } from '@solidjs/router';
import { Accessor, Component, createMemo } from 'solid-js';
import { resolveHomeRoute, resolveSearchRoute, resolveSettingsRoute, resolveSPBrowseRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';

function getCurrent(pathname: string): 'home' | 'search' | 'browse' | 'settings' {
  if (pathname === '/sp') return 'home';
  if (pathname.startsWith('/sp/search')) return 'search';
  if (pathname.startsWith('/sp/settings')) return 'settings';
  return 'browse';
}

const SPBottomNavigation: Component = () => {
  const navigate = useSPNavigate();
  const location = useLocation();
  const current = createMemo<'home' | 'search' | 'browse' | 'settings'>(() => getCurrent(location.pathname));

  return (
    <div
      class='bg-primary-plane z-50 flex h-15 w-full flex-row shadow-[0_-2px_10px_rgba(0,0,0,0.2)]'
      style={{ 'view-transition-name': 'sp-bottom-navigation' }}
    >
      <NavigationItem icon='pixelarticons:home-sharp' currentMode={current} targetMode='home' onClick={() => navigate(resolveHomeRoute())} />

      <NavigationItem icon='pixelarticons:library' currentMode={current} targetMode='browse' onClick={() => navigate(resolveSPBrowseRoute())} />

      <NavigationItem icon='pixelarticons:search' currentMode={current} targetMode='search' onClick={() => navigate(resolveSearchRoute())} />

      <NavigationItem
        icon='pixelarticons:settings-2-sharp'
        currentMode={current}
        targetMode='settings'
        onClick={() => navigate(resolveSettingsRoute())}
      />
    </div>
  );
};

interface NavigationItemProps {
  icon: string;
  targetMode: string;
  currentMode: Accessor<string>;
  onClick?: (e: MouseEvent) => void;
}

const NavigationItem: Component<NavigationItemProps> = (props) => {
  return (
    <div
      class='ripple hover:bg-primary-hover m-1.5 flex flex-1 items-center justify-center overflow-hidden rounded-md'
      classList={{
        'bg-transparent': props.currentMode() !== props.targetMode,
        'bg-primary-selected': props.currentMode() === props.targetMode,
      }}
      onClick={props.onClick}
    >
      <Icon
        icon={props.icon}
        class='scale-150'
        classList={{
          'text-accent': props.currentMode() === props.targetMode,
        }}
      />
    </div>
  );
};

export default SPBottomNavigation;
