// Write global signals (like open/close) and functions of the SPExpandablePlayerBar, which may be called from outside components.
// This will allow SPExpandablePlayerBar to refresh its layout properly on hot reload (otherwise it doesn't.)

import { createSignal } from 'solid-js';

export interface PlayerBarRequest {
  expanded: boolean;
  immediate: boolean;
}

const [spPlayerBarRequest, setSPPlayerBarRequest] = createSignal<PlayerBarRequest | null>(null);

export function openSPExpandablePlayerBar(immediate = false) {
  setSPPlayerBarRequest({ expanded: true, immediate });
}

export function closeSPExpandablePlayerBar(immediate = false) {
  setSPPlayerBarRequest({ expanded: false, immediate });
}

export { spPlayerBarRequest };
