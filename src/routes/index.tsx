import { useNavigate } from '@solidjs/router';
import { onMount } from 'solid-js';
import { fetchBootstrapAppState, loadBootstrapToStore } from '~/features/session/service';

function Index() {
  const navigate = useNavigate();
  // load bootstrap and redirect
  onMount(async () => {
    const bootstrap = await fetchBootstrapAppState();
    loadBootstrapToStore(bootstrap);
    switch (bootstrap.restoreStatus) {
      case 'restored':
        navigate('/home');
        break;
      case 'none':
      case 'offline': // !! temporally assume logouted
      case 'reauth_required': // !! temporally assume logouted
        navigate('/init_login');
        break;
    }
  });

  return <div>Loading...</div>;
}
export default Index;
