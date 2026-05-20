import { onCleanup, onMount, ParentComponent, Show } from 'solid-js';
import { commands, events } from '~/bindings';
import SPBottomNavigation from '~/components/sp/SPBottomNavigation';
import SPExpandablePlayerBar, { openPlayerBar } from '~/components/sp/SPExpandablePlayerBar';
import SPHeader from '~/components/sp/SPHeader';
import { startPlaybackStateSync } from '~/features/playback/service';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';
import { sessionStore } from '~/stores/SessionStore';

const MobileHomeLayout: ParentComponent = (props) => {
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
        <SPHeader />

        <div class='bg-primary-bg relative flex min-h-0 flex-1 flex-col overflow-hidden'>
          <div
            class='absolute inset-0 flex min-h-0 flex-1 flex-col overflow-hidden'
            style={{
              'view-transition-name': 'sp-screen',
              'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
            }}
          >
            {props.children}
          </div>
          <SPExpandablePlayerBar />
        </div>

        <SPBottomNavigation />
      </div>
    </Show>
  );
};

export default MobileHomeLayout;
