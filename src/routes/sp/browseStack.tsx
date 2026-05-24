import { useLocation } from '@solidjs/router';
import { createMemo, ParentComponent, Show } from 'solid-js';
import { resolveSPBrowseRoute } from '~/features/navigation/routes';
import SPBrowseIndex from './browseIndex';

const SPBrowseStack: ParentComponent = (props) => {
  const location = useLocation();
  const browseRoot = resolveSPBrowseRoute();

  const showDetail = createMemo(() => {
    const pathname = location.pathname.replace(/\/+$/, '') || '/';
    return pathname !== browseRoot && pathname.startsWith(`${browseRoot}/`);
  });

  return (
    <div class='relative flex size-full overflow-hidden'>
      <SPBrowseIndex />
      <Show when={showDetail()}>
        <div class='bg-primary-bg absolute inset-0 z-30 flex overflow-y-auto'>{props.children}</div>
      </Show>
    </div>
  );
};

export default SPBrowseStack;
