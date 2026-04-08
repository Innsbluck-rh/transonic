import type { SongResponse } from '~/bindings';
import { resolveAlbumRoute, resolveArtistRoute } from '../navigation/routes';
import type { MenuItem } from './types';

export function buildQueueItemMenuItems(song: SongResponse, _index: number, navigate: (to: string | number) => void): MenuItem[] {
  const albumId = song.albumId;
  const artistId = song.artistId;

  const items: MenuItem[] = [];
  items.push({
    icon: 'material-symbols:playlist-remove',
    label: 'Remove from queue',
    onClick: () => {
      // TODO: implement remove from queue (includes the behavior after removing playing entry)
    },
  });

  if (albumId) {
    items.push({
      icon: 'material-symbols:album',
      label: 'Go to album',
      onClick: () => navigate(resolveAlbumRoute(albumId)),
    });
  }

  if (artistId) {
    items.push({
      icon: 'material-symbols:person',
      label: 'Go to artist',
      onClick: () => navigate(resolveArtistRoute(artistId)),
    });
  }

  return items;
}
