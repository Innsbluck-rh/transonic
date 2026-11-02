import type { QueueSource } from '~/bindings';
import { appendToQueue, insertAfterCurrent, playNow } from '~/features/playback/service';
import type { MenuItem } from './types';

export function buildAlbumMenuItems(source: QueueSource): MenuItem[] {
  return [
    {
      icon: 'icon-[material-symbols--play-arrow]',
      label: 'Play',
      onClick: () => playNow([source]),
    },
    {
      icon: 'icon-[material-symbols--next-plan]',
      label: 'Play Next',
      onClick: () => insertAfterCurrent([source]),
    },
    {
      icon: 'icon-[material-symbols--playlist-add]',
      label: 'Add to Queue',
      onClick: () => appendToQueue([source]),
    },
  ];
}
