import { useNavigate } from '@solidjs/router';
import { onCleanup, onMount, ParentComponent, Show } from 'solid-js';
import Header from '~/components/common/header/Header';
import SPBottomNavigation from '~/components/sp/SPBottomNavigation';
import SPExpandablePlayerBar from '~/components/sp/SPExpandablePlayerBar';
import { startPlaybackStateSync } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';

const HomeLayoutSP: ParentComponent = (props) => {
  let disposed = false;
  let unlisten: (() => void) | undefined;

  onMount(() => {
    void startPlaybackStateSync().then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
        return;
      }

      unlisten = nextUnlisten;
    });
  });

  onCleanup(() => {
    disposed = true;
    unlisten?.();
  });

  const navigate = useNavigate();

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
        <Header shouldShowProfiles={true} titleHref={'/sp'} />
        <SPExpandablePlayerBar>{props.children}</SPExpandablePlayerBar>
        <SPBottomNavigation
          onClickNav={(n) => {
            switch (n) {
              case 'index':
                navigate('/sp');
                break;
              case 'setting':
                // stab
                // navigate('/sp/etting');
                break;
            }
          }}
        />
      </div>
    </Show>
  );
};

export default HomeLayoutSP;
