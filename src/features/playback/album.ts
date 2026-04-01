import { commands, PlaybackQueueEntry } from '~/bindings';
import { hasPlaybackCommandError } from './service';

// TODO: This should replaced by backend command maybe..

export async function replaceQueueWithAlbumAndPlay(albumId: string) {
  const asResult = await commands.getAlbumSongs({ id: albumId });
  if (asResult.status == 'error') {
    return;
  }
  const entries: PlaybackQueueEntry[] = asResult.data.songs.map((song) => {
    return { songId: song.id, title: song.title } as PlaybackQueueEntry;
  });
  // TODO: consider batch this?
  const setQueueResult = await commands.playbackSetQueue({ currentIndex: 0, entries });
  if (hasPlaybackCommandError(setQueueResult)) {
    return;
  }
  const playResult = await commands.playbackPlay();
  hasPlaybackCommandError(playResult);
}

export async function replaceQueueWithFolderAlbumAndPlay(libraryId: string, nodeId: string, albumId: string) {
  const asResult = await commands.getFolderStructureAlbumSongs({ libraryId, nodeId, albumId });
  if (asResult.status == 'error') {
    return;
  }
  const entries: PlaybackQueueEntry[] = asResult.data.songs.map((song) => {
    return { songId: song.id, title: song.title } as PlaybackQueueEntry;
  });
  // TODO: consider batch this?
  const setQueueResult = await commands.playbackSetQueue({ currentIndex: 0, entries });
  if (hasPlaybackCommandError(setQueueResult)) {
    return;
  }
  const playResult = await commands.playbackPlay();
  hasPlaybackCommandError(playResult);
}
