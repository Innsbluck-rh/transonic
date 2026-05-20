import { Icon } from '@iconify-icon/solid';
import { useLocation } from '@solidjs/router';
import { createMemo } from 'solid-js';
import { resolveHomeRoute, resolveSettingsRoute, resolveSPBrowseRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';

function getCurrent(pathname: string): 'home' | 'browse' | 'settings' {
  if (pathname === '/sp') return 'home';
  if (pathname.startsWith('/sp/settings')) return 'settings';
  return 'browse';
}

const SPBottomNavigation = () => {
  const navigate = useSPNavigate();
  const location = useLocation();
  const current = createMemo<'home' | 'browse' | 'settings'>(() => getCurrent(location.pathname));

  return (
    <div
      class='bg-primary-plane z-50 flex h-15 w-full flex-row shadow-[0_-2px_10px_rgba(0,0,0,0.2)]'
      style={{ 'view-transition-name': 'sp-bottom-navigation' }}
    >
      <div
        class='ripple hover:bg-primary-hover m-1.5 flex flex-1 items-center justify-center overflow-hidden rounded-md'
        classList={{
          'bg-transparent': current() !== 'home',
          'bg-primary-selected': current() === 'home',
        }}
        onClick={() => navigate(resolveHomeRoute())}
      >
        <Icon
          icon='pixelarticons:home-sharp'
          class='scale-150'
          classList={{
            'text-accent': current() === 'home',
          }}
        />
      </div>
      <div
        class='ripple hover:bg-primary-hover m-1.5 flex flex-1 items-center justify-center overflow-hidden rounded-md'
        classList={{
          'bg-transparent': current() !== 'browse',
          'bg-primary-selected': current() === 'browse',
        }}
        onClick={() => navigate(resolveSPBrowseRoute())}
      >
        <Icon
          icon='pixelarticons:library'
          class='scale-150'
          classList={{
            'text-accent': current() === 'browse',
          }}
        />
      </div>
      <div
        class='ripple hover:bg-primary-hover m-1.5 flex flex-1 items-center justify-center overflow-hidden rounded-md'
        classList={{
          'bg-transparent': current() !== 'settings',
          'bg-primary-selected': current() === 'settings',
        }}
        onClick={() => navigate(resolveSettingsRoute())}
      >
        <Icon
          icon='pixelarticons:settings-2-sharp'
          class='scale-150'
          classList={{
            'text-accent': current() === 'settings',
          }}
        />
      </div>
    </div>
  );
};

export default SPBottomNavigation;
