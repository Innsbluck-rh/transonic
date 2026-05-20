import { onCleanup, onMount, ParentComponent, Show } from 'solid-js';
import Header from '~/components/common/header/Header';
import PlayerBar from '~/components/player/PlayerBar';
import IndexSideBar from '~/components/sidebar/index/IndexSideBar';
import QueueSideBar from '~/components/sidebar/queue/QueueSideBar';
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
          <Header shouldShowProfiles={true} titleHref={resolveHomeRoute('pc')} />

          <div class='flex h-full w-full flex-row overflow-hidden'>
            <IndexSideBar />
            <div class='flex h-full min-w-0 flex-1 flex-col'>{props.children}</div>
            <QueueSideBar />
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
