/* @refresh reload */
import { attachConsole } from '@tauri-apps/plugin-log';
import { render } from 'solid-js/web';
import App from './App';
import { initializeCjkFontOrder } from './features/font/service';
import { initializeTheme } from './features/theme/service';

type SafeAreaInsetSide = 'top' | 'right' | 'bottom' | 'left';

type SafeAreaInsetsBridge = Record<SafeAreaInsetSide, () => string | undefined>;

declare global {
  interface Window {
    TransonicSafeAreaInsets?: SafeAreaInsetsBridge;
  }
}

const SAFE_AREA_INSET_STORAGE_KEYS = {
  top: 'transonic.safeAreaCssInsetTop',
  right: 'transonic.safeAreaCssInsetRight',
  bottom: 'transonic.safeAreaCssInsetBottom',
  left: 'transonic.safeAreaCssInsetLeft',
} as const;

function initializeSafeAreaInsets() {
  const root = document.documentElement;
  const setSafeAreaInset = (side: SafeAreaInsetSide) => {
    const value = window.TransonicSafeAreaInsets?.[side]?.() ?? window.localStorage.getItem(SAFE_AREA_INSET_STORAGE_KEYS[side]);
    if (/^\d+px$/.test(value ?? '')) {
      root.style.setProperty(`--safe-area-inset-${side}`, value as string);
    }
  };

  try {
    setSafeAreaInset('top');
    setSafeAreaInset('right');
    setSafeAreaInset('bottom');
    setSafeAreaInset('left');
  } catch {
    // The native Android WebView bridge will still update these values after startup.
  }
}

if ('__TAURI_INTERNALS__' in window) {
  void attachConsole().catch((error) => {
    console.error('failed to attach tauri log console', error);
  });
}

initializeSafeAreaInsets();
initializeCjkFontOrder();
initializeTheme();

render(() => <App />, document.getElementById('root') as HTMLElement);
