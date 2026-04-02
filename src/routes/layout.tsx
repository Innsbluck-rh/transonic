import { platform } from '@tauri-apps/plugin-os';
import { ParentComponent, Show } from 'solid-js';

const Layout: ParentComponent = (props) => {
  const pf = platform();
  const isSP = pf === 'android' || pf === 'ios';
  return (
    <div class='bg-primary-plane flex h-dvh w-dvw flex-col'>
      <Show when={isSP}>
        <div class='sp-top-space' />
      </Show>
      {props.children}
      <Show when={isSP}>
        <div class='sp-bottom-space' />
      </Show>
    </div>
  );
};

export default Layout;
