import type { QueueSource } from '~/bindings';
import type { MenuItemDef } from './showMenu';

export interface AlbumMenuCallbacks {
  insertAfterCurrent: (items: QueueSource[]) => void;
  appendToQueue: (items: QueueSource[]) => void;
}

export function buildAlbumMenuItems(source: QueueSource, callbacks: AlbumMenuCallbacks): MenuItemDef[] {
  return [
    {
      label: 'Play Next',
      action: () => callbacks.insertAfterCurrent([source]),
    },
    {
      label: 'Add to Queue',
      action: () => callbacks.appendToQueue([source]),
    },
  ];
}
