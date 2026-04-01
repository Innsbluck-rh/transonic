import { Icon } from '@iconify-icon/solid';
import { useLocation } from '@solidjs/router';
import { createSignal, Match, onCleanup, onMount, ParentComponent, Show, Switch } from 'solid-js';
import Header from '~/components/common/header/Header';
import PlayerBarSP from '~/components/player/PlayerBarSP';
import IndexContent from '~/components/sidebar/index/IndexContent';
import QueueContent from '~/components/sidebar/queue/QueueContent';
import { startPlaybackStateSync } from '~/features/playback/service';
import { sessionStore } from '~/stores/SessionStore';

type SPNavigationState = 'index' | 'main' | 'queue';

const HomeLayoutSP: ParentComponent = (props) => {
  const location = useLocation();
  const [state, setState] = createSignal<SPNavigationState>('main');

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
              <Match when={state() === 'index'}>
                <IndexContent />
              </Match>
              <Match when={state() === 'main'}>
                <div class='flex flex-col min-w-0 flex-1 h-full'>
                  <div class='flex flex-row w-full h-6 items-center px-1 bg-primary-plane border-b border-primary-border'>
                    <p class='archivo text-[10px] text-secondary-text'>{location.pathname}</p>
                  </div>
                  {props.children}
                </div>
              </Match>
              <Match when={state() === 'queue'}>
                <QueueContent />
              </Match>
            </Switch>
          </div>

          <PlayerBarSP />

          <div class='flex flex-row border-t border-primary-border'>
            <div
              class='flex flex-col items-center flex-1 p-2 cursor-pointer hover:bg-primary-hover'
              classList={{ 'bg-primary-hover': state() === 'index' }}
              onClick={() => setState('index')}
            >
              <Icon class='text-3xl text-primary-text' icon='pixelarticons:folder-sharp' />
              <p class='text-xs'>index</p>
            </div>
            <div
              class='flex flex-col items-center flex-1 p-2 cursor-pointer hover:bg-primary-hover'
              classList={{ 'bg-primary-hover': state() === 'main' }}
              onClick={() => setState('main')}
            >
              <Icon class='text-3xl text-primary-text' icon='pixelarticons:grid' />
              <p class='text-xs'>main</p>
            </div>
            <div
              class='flex flex-col items-center flex-1 p-2 cursor-pointer hover:bg-primary-hover'
              classList={{ 'bg-primary-hover': state() === 'queue' }}
              onClick={() => setState('queue')}
            >
              <Icon class='text-3xl text-primary-text' icon='pixelarticons:list' />
              <p class='text-xs'>queue</p>
            </div>
          </div>
        </div>
      </Show>
      <div class='sp-bottom-space' />
    </div>
  );
};

export default HomeLayoutSP;
