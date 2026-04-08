import { ParentComponent, Show } from 'solid-js';
import { isSP } from '~/utils/isSP';

const Layout: ParentComponent = (props) => {
  return (
    <div class='bg-primary-plane flex h-dvh w-dvw flex-col'>
      <Show when={isSP()}>
        <div class='sp-top-space' />
      </Show>
      {props.children}
      <Show when={isSP()}>
        <div class='sp-bottom-space' />
      </Show>
    </div>
  );
};

export default Layout;
