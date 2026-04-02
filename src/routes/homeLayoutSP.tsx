import { useLocation, useNavigate } from '@solidjs/router';
import { Match, onCleanup, onMount, ParentComponent, Show, Switch } from 'solid-js';
import Header from '~/components/common/header/Header';
import PlayerBar from '~/components/player/PlayerBar';
import IndexContent from '~/components/sidebar/index/IndexContent';
import SPBottomNavigation from '~/components/sp/SPBottomNavigation';
import { startPlaybackStateSync } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';
import { setSPNavStore, SPNavStore } from '~/stores/SPNavigationStore';

const HomeLayoutSP: ParentComponent = (props) => {
  const location = useLocation();
  const navigate = useNavigate();

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

  let playerPrevPath: string | undefined;

  return (
    <Show
      when={sessionStore.activeSession}
      fallback={
        <div>
          <p>seems like there's no active profile.</p>
        </div>
      }
    >
      <div class='flex min-h-0 flex-1 flex-col'>
        <Header shouldShowProfiles={true} titleHref='/home' />

        <div class='flex min-h-0 w-full flex-1 flex-row overflow-hidden'>
          <Switch>
            <Match when={SPNavStore.state === 'index'}>
              <IndexContent />
            </Match>
            <Match when={SPNavStore.state === 'main'}>
              <div class='flex h-full min-w-0 flex-1 flex-col'>
                {/* <div class='flex flex-row w-full h-6 items-center bg-primary-plane border-b border-secondary-border'>
                    <p class='archivo text-[10px] text-secondary-text mx-3'>{location.pathname}</p>
                  </div> */}
                {props.children}
              </div>
            </Match>
            {/* <Match when={SPNavStore.state === 'queue'}>
              <QueueContent />
            </Match> */}
          </Switch>
        </div>

        <Show when={!(SPNavStore.state === 'main' && location.pathname === '/sp/player')}>
          <PlayerBar
            iconsVisibility={{
              prev: false,
              playpause: true,
              next: false,
            }}
            onClickRestArea={() => {
              playerPrevPath = location.pathname;
              navigate('/sp/player');
              setSPNavStore('state', 'main');
            }}
          />
        </Show>
        <SPBottomNavigation
          onClickNav={(_nav) => {
            if (playerPrevPath) {
              navigate(playerPrevPath);
              playerPrevPath = undefined;
            }
          }}
        />
      </div>
    </Show>
  );
};

export default HomeLayoutSP;
