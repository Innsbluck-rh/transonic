import { useLocation } from '@solidjs/router';
import { onCleanup, onMount, ParentComponent, Show } from 'solid-js';
import Header from '~/components/common/header/Header';
import PlayerBar from '~/components/player/PlayerBar';
import IndexSideBar from '~/components/sidebar/index/IndexSideBar';
import QueueSideBar from '~/components/sidebar/queue/QueueSideBar';
import { startPlaybackStateSync } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';

const HomeLayout: ParentComponent = (props) => {
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
    <div class='w-dvw h-dvh bg-primary-plane'>
      <Show
        when={sessionStore.activeSession}
        fallback={
          <div>
            <p>seems like there's no active profile.</p>
          </div>
        }
      >
        <div class='flex flex-col h-full'>
          <Header shouldShowProfiles={true} titleHref='/home' />

          <div class='flex flex-row w-full h-full overflow-hidden'>
            <IndexSideBar />
            <div class='flex flex-col min-w-0 flex-1 h-full'>
              <div class='flex flex-row w-full items-center h-6 px-1 bg-primary-plane border-b border-primary-border'>
                <p class='archivo text-[10px] opacity-50'>{location.pathname}</p>
              </div>
              {props.children}
            </div>
            <QueueSideBar />
          </div>

          <PlayerBar />
        </div>
      </Show>
    </div>
  );
};

export default HomeLayout;
