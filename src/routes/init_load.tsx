import { useLocation, useNavigate } from '@solidjs/router';
import { platform } from '@tauri-apps/plugin-os';
import { createSignal, onMount, ParentComponent, Show } from 'solid-js';
import { commands } from '~/bindings';
import { INIT_LOGIN_ROUTE, resolveBootstrapRoute } from '~/features/session/bootstrap';
import { loadBootstrapToStore } from '~/features/session/service';

const InitLoad: ParentComponent = (props) => {
  const location = useLocation();
  const navigate = useNavigate();
  const [loaded, setLoaded] = createSignal(false);
  const pf = platform();
  const routeVariant = pf === 'android' || pf === 'ios' ? 'mobile' : 'desktop';

  onMount(async () => {
    try {
      const result = await commands.bootstrapAppState();
      if (result.status === 'error') {
        console.error(result.error);
        navigate(INIT_LOGIN_ROUTE, { replace: true });
        return;
      }

      const bootstrap = result.data;
      loadBootstrapToStore(bootstrap);

      const nextRoute = resolveBootstrapRoute(bootstrap, routeVariant);
      const currentRoute = `${location.pathname}${location.search}`;
      if (nextRoute !== currentRoute) {
        navigate(nextRoute, { replace: true });
      }
    } catch (invokeError) {
      console.error(invokeError);
      navigate(INIT_LOGIN_ROUTE, { replace: true });
    } finally {
      setLoaded(true);
    }
  });

  return <Show when={loaded()}>{props.children}</Show>;
};

export default InitLoad;
