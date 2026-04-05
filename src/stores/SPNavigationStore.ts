import { createStore } from 'solid-js/store';

export type SPNavigationState = 'index' | 'setting' | 'player';
export type NavDirection = 'forward' | 'backward';

interface SPNavigationStore {
  state: SPNavigationState;
  direction: NavDirection | null;
}

export const [SPNavStore, setSPNavStore] = createStore<SPNavigationStore>({
  state: 'index',
  direction: null,
});
