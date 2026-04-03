import { createStore } from 'solid-js/store';

export type SPNavigationState = 'index' | 'setting';

interface SPNavigationStore {
  state: SPNavigationState;
}

export const [SPNavStore, setSPNavStore] = createStore<SPNavigationStore>({
  state: 'index',
});
