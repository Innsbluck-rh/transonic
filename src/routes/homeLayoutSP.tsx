import { useLocation } from '@solidjs/router';
import { Match, onCleanup, onMount, ParentComponent, Show, Switch } from 'solid-js';
import Header from '~/components/common/header/Header';
import PlayerBar from '~/components/player/PlayerBar';
import IndexContent from '~/components/sidebar/index/IndexContent';
import QueueContent from '~/components/sidebar/queue/QueueContent';
import SPBottomNavigation from '~/components/sp/SPBottomNavigation';
import { startPlaybackStateSync } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';
import { SPNavStore } from '~/stores/SPNavigationStore';

const HomeLayoutSP: ParentComponent = (props) => {
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
    <div class='flex flex-col w-dvw h-dvh bg-primary-plane'>
      <div class='sp-top-space' />

      <Show
        when={sessionStore.activeSession}
        fallback={
          <div>
            <p>seems like there's no active profile.</p>
          </div>
        }
      >
        <div class='flex flex-col min-h-0 flex-1'>
          <Header shouldShowProfiles={true} titleHref='/home' />

          <div class='flex flex-row w-full h-full overflow-hidden'>
            <Switch>
              <Match when={SPNavStore.state === 'index'}>
                <IndexContent />
              </Match>
              <Match when={SPNavStore.state === 'main'}>
                <div class='flex flex-col min-w-0 flex-1 h-full'>
                  {/* <div class='flex flex-row w-full h-6 items-center bg-primary-plane border-b border-secondary-border'>
                    <p class='archivo text-[10px] text-secondary-text mx-3'>{location.pathname}</p>
                  </div> */}
                  {props.children}
                </div>
              </Match>
              <Match when={SPNavStore.state === 'queue'}>
                <QueueContent />
              </Match>
            </Switch>
          </div>

          <PlayerBar
            iconsVisibility={{
              prev: false,
              playpause: true,
              next: false,
            }}
          />

          <SPBottomNavigation />
        </div>
      </Show>
      <div class='sp-bottom-space' />
    </div>
  );
};

export default HomeLayoutSP;
