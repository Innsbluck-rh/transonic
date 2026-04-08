import { platform } from '@tauri-apps/plugin-os';

export function isSP(): boolean {
  const pf = platform();
  return pf === 'android' || pf === 'ios';
}
