// てきとーなストア

import { createStore } from 'solid-js/store';

interface TempStore {
  albumMode: 'grid' | 'list';
}

// 後でrust側設定にもっていくこと！！！
export const [tempStore, setTempStore] = createStore<TempStore>({
  albumMode: 'grid',
});
