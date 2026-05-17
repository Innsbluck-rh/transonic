import { createSignal } from 'solid-js';

export interface PlayerBarRequest {
  expanded: boolean;
  immediate: boolean;
}

export const [playerBarRequest, setPlayerBarRequest] = createSignal<PlayerBarRequest | null>(null);

export function openPlayerBar(immediate = false) {
  setPlayerBarRequest({ expanded: true, immediate });
}

export function closePlayerBar(immediate = false) {
  setPlayerBarRequest({ expanded: false, immediate });
}
