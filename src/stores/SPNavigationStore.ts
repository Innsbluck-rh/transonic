import { createStore } from 'solid-js/store';

export type SPNavigationState = 'index' | 'main';

interface SPNavigationStore {
  state: SPNavigationState;
}

export const [SPNavStore, setSPNavStore] = createStore<SPNavigationStore>({
  state: 'main',
});
