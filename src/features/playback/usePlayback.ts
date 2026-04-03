import { createMemo, createSignal } from 'solid-js';
import { commands, InterruptReason, PlaybackQueueEntry, PlayingState } from '~/bindings';
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

function isSameQueue(q1: PlaybackQueueEntry[], q2: PlaybackQueueEntry[]) {
  return (
    q1.length === q2.length &&
    q1.every((q, i) => {
      return q.songId === q2[i].songId && q.albumId === q2[i].albumId && q.artistId === q2[i].artistId;
    })
  );
}

export function usePlayback() {
  const [previewPositionMs, setPreviewPositionMs] = createSignal<number | null>(null);

  const status = createMemo(() => playbackStore.status);
  const queue = createMemo<PlaybackQueueEntry[]>((prev) => {
    const next = playbackStore.status?.queue ?? [];
    return isSameQueue(prev, next) ? prev : next;
  }, []);
  const currentIndex = createMemo<number | null>(() => status()?.currentIndex ?? null);
  const currentEntry = createMemo<PlaybackQueueEntry | null>(() => {
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

  const play = async () => {
    const result = await commands.playbackPlay();
    hasPlaybackCommandError(result);
  };

  const pause = async () => {
    const result = await commands.playbackPause();
    hasPlaybackCommandError(result);
  };

  const togglePlayPause = async () => {
    if (playingState() === 'playing') {
      await pause();
      return;
    }

    await play();
  };

  const prev = async () => {
    const result = await commands.playbackPrev();
    hasPlaybackCommandError(result);
  };

  const next = async () => {
    const result = await commands.playbackNext();
    hasPlaybackCommandError(result);
  };

  const seek = async (nextPositionMs: number) => {
    const nextDurationMs = durationMs();
    if (nextDurationMs <= 0) {
      setPreviewPositionMs(null);
      return;
    }

    const clampedPositionMs = clampPositionMs(nextPositionMs, nextDurationMs);
    setPreviewPositionMs(null);
    const result = await commands.playbackSeek({ positionMs: clampedPositionMs });
    hasPlaybackCommandError(result);
  };

  const previewSeek = (nextPositionMs: number | null) => {
    setPreviewPositionMs(nextPositionMs);
  };

  const playQueueIndex = async (index: number) => {
    const result = await commands.playbackPlayQueueIndex({ index });
    hasPlaybackCommandError(result);
  };

  const playAlbum = async (albumId: string) => {
    const result = await commands.playbackPlayAlbum({ albumId });
    hasPlaybackCommandError(result);
  };

  const playFolderAlbum = async (libraryId: string, nodeId: string, albumId: string) => {
    const result = await commands.playbackPlayFolderAlbum({ libraryId, nodeId, albumId });
    hasPlaybackCommandError(result);
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
