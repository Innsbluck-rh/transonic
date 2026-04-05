import type { SongResponse } from '~/bindings';
import { resolveAlbumRoute, resolveArtistRoute } from '../navigation/routes';
import type { MenuItemDef } from './showMenu';

/** Queue item-specific context menu items such as remove from queue (to be implemented) */
export function buildQueueItemMenuItems(_song: SongResponse, _index: number, navigate: (to: string | number) => void): MenuItemDef[] {
  const albumId = _song.albumId;
  const artistId = _song.artistId;

  const items: MenuItemDef[] = [];

  if (albumId) {
    items.push({
      label: 'Go to album',
      action: () => navigate(resolveAlbumRoute(albumId, 'desktop')),
    });
  }

  if (artistId) {
    items.push({
      label: 'Go to artist',
      action: () => navigate(resolveArtistRoute(artistId, 'desktop')),
    });
  }

  return items;
}
