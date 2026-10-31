import type { ActiveSession } from '~/bindings';

export function activeSession(profileId: string): ActiveSession {
  return {
    profileId,
    normalizedServerUrl: 'https://music.example',
    username: 'user',
    authKind: 'password',
    apiVersion: '1.16.1',
    serverType: null,
    serverVersion: null,
    capabilityMatrix: {
      openSubsonic: true,
      rawExtensions: [],
      apiKeyAuth: false,
      indexBasedQueue: false,
      playbackReport: false,
      transcoding: false,
      transcodeOffset: false,
      songLyrics: false,
    },
  };
}
