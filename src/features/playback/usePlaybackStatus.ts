import { createMemo } from 'solid-js';
import { connectStore } from '~/stores/ConnectStore';
import { playbackStore } from '~/stores/PlaybackStore';
import { resolvePlaybackDisplayStatus, usesLocalPlaybackDiagnostics } from './sharedPlaybackStatus';

export function usePlaybackStatus() {
  return createMemo(() =>
    resolvePlaybackDisplayStatus(playbackStore.status, connectStore.runtime.connected, connectStore.sharedPlayback, connectStore.runtime.deviceId)
  );
}

export function useLocalPlaybackDiagnosticsAvailable() {
  return createMemo(
    () => !connectStore.runtime.connected || usesLocalPlaybackDiagnostics(connectStore.sharedPlayback, connectStore.runtime.deviceId)
  );
}
