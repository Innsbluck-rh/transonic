export type AlbumListContext =
  | 'random'
  | 'newest'
  | 'frequent'
  | 'recent'
  | 'starred'
  | 'alphabetical_by_name'
  | 'alphabetical_by_artist'
  | 'by_year'
  | 'by_genre';

export type AlbumListRequest = {
  context: AlbumListContext;
  size?: number;
  offset?: number;
  fromYear?: number;
  toYear?: number;
  genre?: string;
  musicFolderId?: string;
};

export type AlbumListItem = {
  id: string;
  name: string;
  artist?: string | null;
  coverArtId?: string | null;
  year?: number | null;
};

export type AlbumListResponse = {
  context: AlbumListContext;
  albums: AlbumListItem[];
};

export type CoverArtRequest = {
  coverArtId: string;
  size?: number;
};

export type CoverArtResponse = {
  dataUrl: string;
};
