/* @refresh reload */
import { attachConsole } from '@tauri-apps/plugin-log';
import { render } from 'solid-js/web';
import App from './App';

if ('__TAURI_INTERNALS__' in window) {
  void attachConsole().catch((error) => {
    console.error('failed to attach tauri log console', error);
  });
}

render(() => <App />, document.getElementById('root') as HTMLElement);
