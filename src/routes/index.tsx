import { useNavigate } from '@solidjs/router';
import { onMount } from 'solid-js';
import { commands } from '~/bindings';
import { resolveBootstrapRoute } from '~/features/session/bootstrap';
import { loadBootstrapToStore } from '~/features/session/service';

function Index() {
  const navigate = useNavigate();
  // load bootstrap and redirect
  onMount(async () => {
    try {
      const result = await commands.bootstrapAppState();
      if (result.status === 'error') {
        console.error(result.error);
        navigate('/init_login');
        return;
      }

      const bootstrap = result.data;
      loadBootstrapToStore(bootstrap);
      navigate(resolveBootstrapRoute(bootstrap));
    } catch (invokeError) {
      console.error(invokeError);
      navigate('/init_login');
    }
  });

  return <div>Loading...</div>;
}
export default Index;
