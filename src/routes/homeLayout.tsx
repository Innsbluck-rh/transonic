import { useLocation } from '@solidjs/router';
import { onCleanup, onMount, ParentComponent, Show } from 'solid-js';
import Header from '~/components/common/header/Header';
import PlayerBar from '~/components/player/PlayerBar';
import IndexSideBar from '~/components/sidebar/index/IndexSideBar';
import QueueSideBar from '~/components/sidebar/queue/QueueSideBar';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { startPlaybackStateSync } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';

const HomeLayout: ParentComponent = (props) => {
  const navigate = useSPNavigate();
  const location = useLocation();

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
    <div class='bg-primary-plane border-secondary-border h-dvh w-dvw'>
      <Show
        when={sessionStore.activeSession}
        fallback={
          <div>
            <p>seems like there's no active profile.</p>
          </div>
        }
      >
        <div class='flex h-full flex-col'>
          <Header shouldShowProfiles={true} titleHref='/home' />

          <div class='flex h-full w-full flex-row overflow-hidden'>
            <IndexSideBar />
            <div class='flex h-full min-w-0 flex-1 flex-col'>
              {/*<div class='bg-primary-plane border-primary-border flex h-6 w-full flex-row items-center border-b'>
                <p class='archivo text-secondary-text mx-4 text-[10px]'>{location.pathname}</p>
              </div>*/}
              {props.children}
            </div>
            <QueueSideBar />
          </div>

          <PlayerBar
            onClickTitle={(entry) => {
              if (!entry) return;
              // go to album
              navigate(`/browse/album/${entry?.albumId}`);
            }}
            onClickArtist={(entry) => {
              if (!entry) return;
              // go to album
              navigate(`/browse/artists/${entry?.artistId}`);
            }}
          />
        </div>
      </Show>
    </div>
  );
};

export default HomeLayout;
