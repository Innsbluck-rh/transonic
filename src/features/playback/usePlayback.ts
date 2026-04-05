import { createMemo, createSignal } from 'solid-js';
import { commands, InterruptReason, PlayingState, type SongResponse } from '~/bindings';
import { hasPlaybackCommandError } from './service';

import { playbackStore } from '~/stores/PlaybackStore';

function formatTime(totalSeconds: number) {
  const safeSeconds = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);
  const seconds = safeSeconds % 60;

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }

  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

function clampPositionMs(positionMs: number, durationMs: number) {
  if (durationMs <= 0) {
    return Math.max(0, positionMs);
  }

  return Math.min(Math.max(0, positionMs), durationMs);
}

function isControlDisabledForState(state?: PlayingState) {
  return !state || state === 'idle' || state === 'error' || state === 'interrupted';
}

function isSeekDisabledForState(state?: PlayingState) {
  return !state || state === 'idle' || state === 'error' || state === 'interrupted';
}

function interruptLabelForReason(reason: InterruptReason | null) {
  switch (reason) {
    case 'initial_load':
      return '[loading...]';
    case 'stream_buffering_stall':
      return '[buffering...]';
    case 'seeking':
      return '[seeking...]';
    case 'full_reload':
      return '[reloading...]';
    default:
      return '';
  }
}

function isSameQueue(q1: SongResponse[], q2: SongResponse[]) {
  return (
    q1.length === q2.length &&
    q1.every((q, i) => {
      return q.id === q2[i].id;
    })
  );
}

export function usePlayback() {
  const [previewPositionMs, setPreviewPositionMs] = createSignal<number | null>(null);

  const status = createMemo(() => playbackStore.status);
  const queue = createMemo<SongResponse[]>((prev) => {
    const next = playbackStore.status?.queue ?? [];
    return isSameQueue(prev, next) ? prev : next;
  }, []);
  const currentIndex = createMemo<number | null>(() => status()?.currentIndex ?? null);
  const currentEntry = createMemo<SongResponse | null>(() => {
    const index = currentIndex();
    if (index === null) {
      return null;
    }

    return queue()[index] ?? null;
  });
  const playingState = createMemo<PlayingState>(() => status()?.playingState ?? 'idle');
  const interruptReason = createMemo<InterruptReason | null>(() => status()?.interruptReason ?? null);
  const isInterrupted = createMemo(() => playingState() === 'interrupted');
  const isControlDisabled = createMemo(() => isControlDisabledForState(playingState()));
  const durationMs = createMemo(() => (currentEntry()?.duration ?? 0) * 1000);
  const currentPositionMs = createMemo(() => {
    const nextPreviewPositionMs = previewPositionMs();
    if (nextPreviewPositionMs !== null) {
      return clampPositionMs(nextPreviewPositionMs, durationMs());
    }

    const nextPendingSeekPositionMs = status()?.pendingSeekPositionMs ?? null;
    if (nextPendingSeekPositionMs !== null) {
      return clampPositionMs(nextPendingSeekPositionMs, durationMs());
    }

    return clampPositionMs(status()?.currentPositionMs ?? 0, durationMs());
  });
  const progressPercent = createMemo(() => {
    const nextDurationMs = durationMs();
    if (nextDurationMs <= 0) {
      return 0;
    }

    return (currentPositionMs() / nextDurationMs) * 100;
  });
  const canSeek = createMemo(() => !isSeekDisabledForState(playingState()) && durationMs() > 0);
  const interruptLabel = createMemo(() => interruptLabelForReason(interruptReason()));
  const currentPositionText = createMemo(() => formatTime(currentPositionMs() / 1000));
  const durationText = createMemo(() => formatTime(durationMs() / 1000));

  const play = () => {
    commands.playbackPlay().then(hasPlaybackCommandError);
  };

  const pause = () => {
    commands.playbackPause().then(hasPlaybackCommandError);
  };

  const togglePlayPause = () => {
    if (playingState() === 'playing') {
      pause();
      return;
    }

    play();
  };

  const prev = () => {
    commands.playbackPrev().then(hasPlaybackCommandError);
  };

  const next = () => {
    commands.playbackNext().then(hasPlaybackCommandError);
  };

  const seek = (nextPositionMs: number) => {
    const nextDurationMs = durationMs();
    if (nextDurationMs <= 0) {
      setPreviewPositionMs(null);
      return;
    }

    const clampedPositionMs = clampPositionMs(nextPositionMs, nextDurationMs);
    setPreviewPositionMs(null);
    commands.playbackSeek({ positionMs: clampedPositionMs }).then(hasPlaybackCommandError);
  };

  const previewSeek = (nextPositionMs: number | null) => {
    setPreviewPositionMs(nextPositionMs);
  };

  const playQueueIndex = (index: number) => {
    commands.playbackPlayQueueIndex({ index }).then(hasPlaybackCommandError);
  };

  const playAlbum = (albumId: string) => {
    commands.playbackPlayAlbum({ albumId }).then(hasPlaybackCommandError);
  };

  const playFolderAlbum = (libraryId: string, nodeId: string, albumId: string) => {
    commands.playbackPlayFolderAlbum({ libraryId, nodeId, albumId }).then(hasPlaybackCommandError);
  };

  const isQueueIndexActive = (index: number) => currentIndex() === index;
  const queueIndexProgressPercent = (index: number) => (isQueueIndexActive(index) ? progressPercent() : 0);

  return {
    queue,
    currentIndex,
    currentEntry,
    playingState,
    interruptReason,
    isInterrupted,
    isControlDisabled,
    canSeek,
    currentPositionMs,
    durationMs,
    progressPercent,
    currentPositionText,
    durationText,
    interruptLabel,
    isQueueIndexActive,
    queueIndexProgressPercent,
    play,
    pause,
    togglePlayPause,
    prev,
    next,
    seek,
    previewSeek,
    playQueueIndex,
    playAlbum,
    playFolderAlbum,
  };
}
