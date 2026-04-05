import { useLocation } from '@solidjs/router';
import { createMemo, onCleanup, onMount, ParentComponent, Show } from 'solid-js';
import { commands, events } from '~/bindings';
import Header from '~/components/common/header/Header';
import SPBottomNavigation from '~/components/sp/SPBottomNavigation';
import SPExpandablePlayerBar, { openPlayerBar } from '~/components/sp/SPExpandablePlayerBar';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { startPlaybackStateSync } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';

const HomeLayoutSP: ParentComponent = (props) => {
  const location = useLocation();
  const spNavigate = useSPNavigate();

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

    // Android: 通知タップでプレイヤーバーを展開するイベントリスナー
    void events.mediaNotificationTap
      .listen(() => {
        openPlayerBar();
      })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }

        const prev = unlisten;
        unlisten = () => {
          prev?.();
          nextUnlisten();
        };
      });

    // Android: コールドスタート時に通知タップで起動された場合の処理
    void commands.consumePendingNotificationTap().then((pending) => {
      if (pending && !disposed) {
        openPlayerBar();
      }
    });
  });

  onCleanup(() => {
    disposed = true;
    unlisten?.();
  });

  const canBack = createMemo<boolean>(() => !(location.pathname === '/sp' || location.pathname.startsWith('/sp/index')));

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
        <SPExpandablePlayerBar>
          <div class='flex min-h-0 flex-1 flex-col'>
            {/* <div class='border-primary-border flex h-12 w-full flex-row items-center gap-3 border-b px-3'>
              <Show when={canBack()}>
                <Icon
                  class='scale-200'
                  icon='material-symbols:arrow-back'
                  onClick={() => {
                    if (canBack()) spNavigate(-1);
                  }}
                />
              </Show>
              <Heading3>{location.pathname}</Heading3>
            </div> */}
            <div class='flex min-h-0 flex-1 flex-col'>{props.children}</div>
          </div>
        </SPExpandablePlayerBar>
        <SPBottomNavigation
          onClickNav={(nav, prevNav) => {
            switch (nav) {
              case 'index':
                if (location.pathname !== '/sp' && prevNav !== 'player')
                  spNavigate('/sp', {
                    replace: true,
                  });
                break;
              case 'player':
                openPlayerBar();
                break;
              case 'setting':
                // stab
                // navigate('/sp/setting');
                break;
            }
          }}
        />
      </div>
    </Show>
  );
};

export default HomeLayoutSP;
