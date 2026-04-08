import type { QueueSource } from '~/bindings';
import type { MenuItem } from './types';

export interface AlbumMenuCallbacks {
  insertAfterCurrent: (items: QueueSource[]) => void;
  appendToQueue: (items: QueueSource[]) => void;
}

export function buildAlbumMenuItems(source: QueueSource, callbacks: AlbumMenuCallbacks): MenuItem[] {
  return [
    {
      icon: 'material-symbols:next-plan',
      label: 'Play Next',
      onClick: () => callbacks.insertAfterCurrent([source]),
    },
    {
      icon: 'material-symbols:playlist-add',
      label: 'Add to Queue',
      onClick: () => callbacks.appendToQueue([source]),
    },
  ];
}
