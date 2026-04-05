import type { QueueSource, SongResponse } from '~/bindings';
import type { MenuItemDef } from './showMenu';

export interface SongMenuCallbacks {
  insertAfterCurrent: (items: QueueSource[]) => void;
  appendToQueue: (items: QueueSource[]) => void;
}

export function buildSongMenuItems(song: SongResponse, callbacks: SongMenuCallbacks): MenuItemDef[] {
  return [
    {
      label: 'Play Next',
      action: () => callbacks.insertAfterCurrent([{ type: 'songs', songs: [song] }]),
    },
    {
      label: 'Add to Queue',
      action: () => callbacks.appendToQueue([{ type: 'songs', songs: [song] }]),
    },
  ];
}
