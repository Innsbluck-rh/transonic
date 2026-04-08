import type { QueueSource, SongResponse } from '~/bindings';
import type { MenuItem } from './types';

export interface SongMenuCallbacks {
  insertAfterCurrent: (items: QueueSource[]) => void;
  appendToQueue: (items: QueueSource[]) => void;
}

export function buildSongMenuItems(song: SongResponse, callbacks: SongMenuCallbacks): MenuItem[] {
  return [
    {
      icon: 'material-symbols:next-plan',
      label: 'Play Next',
      onClick: () => callbacks.insertAfterCurrent([{ type: 'songs', songs: [song] }]),
    },
    {
      icon: 'material-symbols:playlist-add',
      label: 'Add to Queue',
      onClick: () => callbacks.appendToQueue([{ type: 'songs', songs: [song] }]),
    },
  ];
}
