import { useLocation, useNavigate } from '@solidjs/router';
import { createSignal, onMount, ParentComponent, Show } from 'solid-js';
import { commands } from '~/bindings';
import Header from '~/components/header/Header';
import IndexSideBar from '~/components/index/IndexSideBar';
import PlayerBar from '~/components/player/PlayerBar';
import { HOME_ROUTE, INIT_LOGIN_ROUTE, isHomeErrorRoute, resolveBootstrapRoute } from '~/features/session/bootstrap';
import { loadBootstrapToStore } from '~/features/session/service';
import { sessionStore } from '~/stores/SessionStore';

const HomeLayout: ParentComponent = (props) => {
  const location = useLocation();
  const navigate = useNavigate();

  const [loaded, setLoaded] = createSignal(false);
  const shouldRenderErrorRoute = () => isHomeErrorRoute(location.pathname);

  const loadBootStrap = async (): Promise<boolean> => {
    try {
      const result = await commands.bootstrapAppState();
      if (result.status === 'error') {
        console.error(result.error);
        return false;
      }

      const bootstrap = result.data;
      loadBootstrapToStore(bootstrap);
      const nextRoute = resolveBootstrapRoute(bootstrap);

      if (nextRoute === INIT_LOGIN_ROUTE) {
        navigate(nextRoute);
        return false;
      }

      if (nextRoute === HOME_ROUTE) {
        if (shouldRenderErrorRoute()) {
          navigate(nextRoute);
        }
        return true;
      }

      if (nextRoute !== location.pathname) {
        navigate(nextRoute);
      }

      return true;
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
    <div class='w-dvw h-dvh bg-zinc-50'>
      <Show when={loaded()}>
        <Show
          when={shouldRenderErrorRoute()}
          fallback={
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
                  {props.children}
                </div>

                <PlayerBar />
              </div>
            </Show>
          }
        >
          {props.children}
        </Show>
      </Show>
    </div>
  );
};

export default HomeLayout;
