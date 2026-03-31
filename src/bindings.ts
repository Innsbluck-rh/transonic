export * from './bindings.gen';

import { commands as rawCommands } from './bindings.gen';

const session = {
  bootstrapAppState: rawCommands.bootstrapAppState,
  connectServerProfile: rawCommands.connectServerProfile,
  deleteServerProfile: rawCommands.deleteServerProfile,
} as const;

const index = {
  getMusicFolders: rawCommands.getMusicFolders,
  getArtistIndexes: rawCommands.getArtistIndexes,
  getFolderStructureRoots: rawCommands.getFolderStructureRoots,
} as const;

const browse = {
  getMusicDirectory: rawCommands.getMusicDirectory,
  getFolderStructureAlbums: rawCommands.getFolderStructureAlbums,
} as const;

const albums = {
  getAlbumList: rawCommands.getAlbumList,
} as const;

const coverArt = {
  getCoverArt: rawCommands.getCoverArt,
} as const;

export const commandGroups = {
  session,
  index,
  browse,
  albums,
  coverArt,
} as const;

export const commands = {
  ...rawCommands,
  ...commandGroups,
} as const;

export type CommandGroups = typeof commandGroups;
