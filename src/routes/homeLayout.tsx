import { useNavigate } from '@solidjs/router';
import { createSignal, onMount, ParentComponent, Show } from 'solid-js';
import ErrorMsg from '~/components/common/ErrorMsg';
import Header from '~/components/header/Header';
import NavigationSideBar from '~/components/navigation/NavigationSideBar';
import PlayerBar from '~/components/player/PlayerBar';
import { fetchBootstrapAppState, loadBootstrapToStore } from '~/features/session/service';
import { sessionStore } from '~/stores/session/SessionStore';

const HomeLayout: ParentComponent = (props) => {
  const navigate = useNavigate();

  const [loaded, setLoaded] = createSignal(false);
  const loadBootStrap = async (): Promise<boolean> => {
    try {
      const bootstrap = await fetchBootstrapAppState();
      loadBootstrapToStore(bootstrap);

      switch (bootstrap.restoreStatus) {
        case 'restored':
          return true;
        case 'none':
          if (!bootstrap.activeSession) {
            navigate('/init_login');
          } else {
            return true;
          }
          break;
        case 'offline':
        case 'reauth_required':
          navigate('/init_login');
          break;
      }
    } catch (invokeError) {
      console.error(invokeError);
    }
    return false;
  };

  onMount(async () => {
    const loadSuccess = await loadBootStrap();
    setLoaded(loadSuccess);
  });

  return (
    <div class='w-dvw h-dvh'>
      <Show when={loaded()} fallback={<ErrorMsg>Error!</ErrorMsg>}>
        <Show
          when={sessionStore.activeSession}
          fallback={
            <div>
              <p>seems like there's no active profile.</p>
            </div>
          }
        >
          <div class='flex flex-col h-full'>
            <Header shouldShowProfiles={true} />

            <div class='flex flex-row w-full h-full overflow-hidden'>
              <NavigationSideBar />
              {props.children}
            </div>

            <PlayerBar />
          </div>
        </Show>
      </Show>
    </div>
  );
};

export default HomeLayout;
