import { createStore } from 'solid-js/store';

interface SPScrollPaddingStore {
  collapsedPlayerHeight: number;
}

export const [SPScrollPaddingStore, setSPScrollPaddingStore] = createStore<SPScrollPaddingStore>({
  collapsedPlayerHeight: 0,
});
