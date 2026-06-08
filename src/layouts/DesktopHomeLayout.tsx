import { onCleanup, onMount, ParentComponent, Show } from 'solid-js';
import Header from '~/components/common/header/Header';
import PlayerBar from '~/components/player/PlayerBar';
import LeftSideBar from '~/components/sidebar/LeftSideBar';
import RightSidebar from '~/components/sidebar/RightSideBar';
import ConnectSync from '~/features/connect/ConnectSync';
import { resolveAlbumRoute, resolveArtistRoute, resolveHomeRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { startPlaybackStateSync } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';

const DesktopHomeLayout: ParentComponent = (props) => {
  const navigate = useSPNavigate();

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void startPlaybackStateSync().then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
        return;
      }

      unlisten = nextUnlisten;
    });

    onCleanup(() => {
      disposed = true;
      unlisten?.();
    });
  });

  return (
    <div class='bg-primary-bg border-secondary-border h-dvh w-dvw'>
      <Show
        when={sessionStore.activeSession}
        fallback={
          <div>
            <p>seems like there's no active profile.</p>
          </div>
        }
      >
        <div class='flex h-full flex-col'>
          <ConnectSync />
          <Header shouldShowProfiles={true} titleHref={resolveHomeRoute('pc')} />

          <div class='flex size-full flex-row overflow-hidden'>
            <LeftSideBar />
            <div class='bg-primary-plane relative flex h-full min-w-0 flex-1'>
              <div class='border-secondary-border absolute inset-0 m-2 flex flex-col overflow-hidden rounded-xl border'>{props.children}</div>
            </div>
            <RightSidebar />
          </div>

          <PlayerBar
            onClickTitle={(entry) => {
              if (!entry?.albumId) return;
              navigate(resolveAlbumRoute(entry.albumId, 'pc'));
            }}
            onClickArtist={(entry) => {
              if (!entry?.artistId) return;
              navigate(resolveArtistRoute(entry.artistId, 'pc'));
            }}
          />
        </div>
      </Show>
    </div>
  );
};

export default DesktopHomeLayout;
