export type MusicFolderSummary = {
  id: string;
  name: string;
};

export type MusicFoldersResponse = {
  musicFolders: MusicFolderSummary[];
};

export type ArtistIndexItem = {
  id: string;
  name: string;
};

export type ArtistIndexesResponse = {
  artists: ArtistIndexItem[];
};

export type ArtistIndexesRequest = {
  musicFolderId?: string;
};

export type MusicDirectoryRequest = {
  id: string;
};

export type MusicDirectoryChild = {
  id: string;
  name: string;
  artist?: string | null;
  coverArtId?: string | null;
  year?: number | null;
  isDirectory: boolean;
};

export type MusicDirectoryResponse = {
  id: string;
  name: string;
  children: MusicDirectoryChild[];
};
