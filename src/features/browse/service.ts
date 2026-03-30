import {
  commands,
  type ArtistIndexesRequest,
  type ArtistIndexesResponse,
  type MusicDirectoryRequest,
  type MusicDirectoryResponse,
  type MusicFoldersResponse,
} from '~/bindings';

export async function fetchMusicFolders(): Promise<MusicFoldersResponse> {
  const result = await commands.getMusicFolders();
  if (result.status === 'error') {
    throw new Error(result.error);
  }

  return result.data;
}

export async function fetchArtistIndexes(payload: ArtistIndexesRequest | null = null): Promise<ArtistIndexesResponse> {
  const result = await commands.getArtistIndexes(payload);
  if (result.status === 'error') {
    throw new Error(result.error);
  }

  return result.data;
}

export async function fetchMusicDirectory(payload: MusicDirectoryRequest): Promise<MusicDirectoryResponse> {
  const result = await commands.getMusicDirectory(payload);
  if (result.status === 'error') {
    throw new Error(result.error);
  }

  return result.data;
}
