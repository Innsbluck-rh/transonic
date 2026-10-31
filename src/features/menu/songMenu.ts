import type { SongResponse } from '~/bindings';
import { appendToQueue, insertAfterCurrent } from '~/features/playback/service';
import type { MenuItem } from './types';

export function buildSongMenuItems(song: SongResponse): MenuItem[] {
  return [
    {
      icon: 'icon-[material-symbols--next-plan]',
      label: 'Play Next',
      onClick: () => insertAfterCurrent([{ type: 'songs', songs: [song] }]),
    },
    {
      icon: 'icon-[material-symbols--playlist-add]',
      label: 'Add to Queue',
      onClick: () => appendToQueue([{ type: 'songs', songs: [song] }]),
    },
  ];
}
