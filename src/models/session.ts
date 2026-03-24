

export type AuthKind = "password" | "api_key";

// TBD: ここの共通化の要・不要
export interface SessionBase {
  profileId: string;
  normalizedServerUrl: string;
  authKind: AuthKind;
  username: string;
}

export type LastConnectionState = "never" | "ok" | "offline" | "reauth_required";

export type SavedProfileSummary = SessionBase & {
  displayName: string;
  lastConnectionState: LastConnectionState;
  isActive: boolean;
};

type CapabilityMatrix = {
  openSubsonic: boolean;
  rawExtensions: OpenSubsonicExtension[];
  apiKeyAuth: boolean;
  indexBasedQueue: boolean;
  playbackReport: boolean;
  transcoding: boolean;
  transcodeOffset: boolean;
  songLyrics: boolean;
};

type OpenSubsonicExtension = {
  name: string;
  versions: number[];
};

export type ActiveSession = SessionBase & {
  apiVersion: string;
  serverType?: string | null;
  serverVersion?: string | null;
  capabilityMatrix: CapabilityMatrix;
};
