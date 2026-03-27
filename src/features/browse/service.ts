import { invoke } from '@tauri-apps/api/core';
import { ArtistIndexesRequest, ArtistIndexesResponse, MusicDirectoryRequest, MusicDirectoryResponse, MusicFoldersResponse } from '~/models/browse';

export async function fetchMusicFolders() {
  return invoke<MusicFoldersResponse>('get_music_folders');
}

export async function fetchArtistIndexes(payload?: ArtistIndexesRequest) {
  if (!payload) {
    return invoke<ArtistIndexesResponse>('get_artist_indexes');
  }

  return invoke<ArtistIndexesResponse>('get_artist_indexes', {
    payload,
  });
}

export async function fetchMusicDirectory(payload: MusicDirectoryRequest) {
  return invoke<MusicDirectoryResponse>('get_music_directory', {
    payload,
  });
}
