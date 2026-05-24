import { commands, type SongResponse } from '~/bindings';
import { resolveAlbumRoute, resolveArtistRoute } from '../navigation/routes';
import { hasPlaybackCommandError } from '../playback/service';
import type { MenuItem } from './types';

export function buildQueueItemMenuItems(song: SongResponse, index: () => number, navigate: (to: string | number) => void): MenuItem[] {
  const albumId = song.albumId;
  const artistId = song.artistId;

  const items: MenuItem[] = [];
  items.push({
    icon: 'material-symbols:playlist-remove',
    label: 'Remove from queue',
    onClick: () => {
      commands.playbackRemoveQueueIndex({ index: index() }).then(hasPlaybackCommandError);
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
