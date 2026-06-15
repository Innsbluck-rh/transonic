import { createMemo, createSignal } from 'solid-js';
import { commands, type InterruptReason, type PlayingState, type QueueSource, type SongResponse } from '~/bindings';
import { setPlaybackSetting, settingsStore } from '~/features/settings/service';
import { connectStore } from '~/stores/ConnectStore';
import { formatClockTime } from '~/utils/duration';
import { hasPlaybackCommandError } from './service';

import { playbackStore } from '~/stores/PlaybackStore';
import { usePlaybackStatus } from './usePlaybackStatus';

function clampPositionMs(positionMs: number, durationMs: number) {
  if (durationMs <= 0) {
    return Math.max(0, positionMs);
  }

  return Math.min(Math.max(0, positionMs), durationMs);
}

function clampVolume(volume: number) {
  if (!Number.isFinite(volume)) {
    return 1;
  }

  return Math.min(Math.max(volume, 0), 1);
}

function isControlDisabledForState(state?: PlayingState) {
  return !state || state === 'idle' || state === 'interrupted';
}

function isSeekDisabledForState(state?: PlayingState) {
  return !state || state === 'idle' || state === 'interrupted';
}

function isPositionAdvancingState(state?: PlayingState) {
  return state === 'playing';
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
  const [previewVolume, setPreviewVolume] = createSignal<number | null>(null);

  const status = usePlaybackStatus();
  const authoritativeReceivedAtMs = createMemo(() =>
    connectStore.runtime.connected && connectStore.sharedPlayback ? connectStore.sharedPlaybackReceivedAtMs : playbackStore.authoritativeReceivedAtMs
  );
  const clockMs = createMemo(() => playbackStore.clockMs);
  const queue = createMemo<SongResponse[]>((prev) => {
    const next = status()?.queue ?? [];
    return isSameQueue(prev, next) ? prev : next;
  }, []);
  const currentIndex = createMemo<number | null>(() => status()?.currentIndex ?? null);
  const playNextQueueLen = createMemo<number>(() => status()?.playNextQueueLen ?? 0);
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
    const nextDurationMs = durationMs();
    const nextPreviewPositionMs = previewPositionMs();
    if (nextPreviewPositionMs !== null) {
      return clampPositionMs(nextPreviewPositionMs, nextDurationMs);
    }

    const nextPendingSeekPositionMs = status()?.pendingSeekPositionMs ?? null;
    if (nextPendingSeekPositionMs !== null) {
      return clampPositionMs(nextPendingSeekPositionMs, nextDurationMs);
    }

    const basePositionMs = status()?.currentPositionMs ?? 0;
    if (!isPositionAdvancingState(playingState())) {
      return clampPositionMs(basePositionMs, nextDurationMs);
    }

    const baseReceivedAtMs = authoritativeReceivedAtMs();
    if (baseReceivedAtMs === null) {
      return clampPositionMs(basePositionMs, nextDurationMs);
    }

    const elapsedMs = Math.max(0, clockMs() - baseReceivedAtMs);
    return clampPositionMs(basePositionMs + elapsedMs, nextDurationMs);
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
  const currentPositionText = createMemo(() => formatClockTime(currentPositionMs() / 1000));
  const durationText = createMemo(() => formatClockTime(durationMs() / 1000));
  const gaplessStatus = createMemo(() => status()?.gaplessStatus ?? null);
  const volume = createMemo(() => previewVolume() ?? clampVolume(settingsStore.playback.volume));

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

  const setVolume = (nextVolume: number) => {
    const clampedVolume = clampVolume(nextVolume);
    setPreviewVolume(null);
    commands.playbackSetVolume({ volume: clampedVolume }).then(hasPlaybackCommandError);
    void setPlaybackSetting('volume', clampedVolume);
  };

  const previewVolumeChange = (nextVolume: number) => {
    const clampedVolume = clampVolume(nextVolume);
    setPreviewVolume(clampedVolume);
    commands.playbackSetVolume({ volume: clampedVolume }).then(hasPlaybackCommandError);
  };

  const commitPreviewedVolume = (nextVolume: number) => {
    const clampedVolume = clampVolume(nextVolume);
    setPreviewVolume(null);
    void setPlaybackSetting('volume', clampedVolume);
  };

  const playQueueIndex = (index: number) => {
    commands.playbackPlayQueueIndex({ index }).then(hasPlaybackCommandError);
  };

  const playAlbum = (albumId: string) => {
    commands
      .playbackSetQueue({
        items: [{ type: 'album', albumId }],
        currentIndex: null,
        autoPlay: true,
      })
      .then(hasPlaybackCommandError);
  };

  const playFolderAlbum = (libraryId: string, nodeId: string, albumId: string) => {
    commands
      .playbackSetQueue({
        items: [{ type: 'folder_album', libraryId, nodeId, albumId }],
        currentIndex: null,
        autoPlay: true,
      })
      .then(hasPlaybackCommandError);
  };

  const playSongs = (songs: SongResponse[], startIndex: number) => {
    commands
      .playbackSetQueue({
        items: [{ type: 'songs', songs }],
        currentIndex: songs.length ? startIndex : null,
        autoPlay: true,
      })
      .then(hasPlaybackCommandError);
  };

  const insertAfterCurrent = (items: QueueSource[]) => {
    commands.playbackInsertAfterCurrent({ items }).then(hasPlaybackCommandError);
  };

  const appendToQueue = (items: QueueSource[]) => {
    commands.playbackAppendToQueue({ items }).then(hasPlaybackCommandError);
  };

  const clearQueue = () => {
    commands.playbackClearQueue().then(hasPlaybackCommandError);
  };

  const isQueueIndexActive = (index: number) => currentIndex() === index;
  const queueIndexProgressPercent = (index: number) => (isQueueIndexActive(index) ? progressPercent() : 0);
  const isPlayNextQueueIndex = (index: number) => {
    const current = currentIndex();
    const start = current === null ? 0 : Math.min(current + 1, queue().length);
    const end = Math.min(start + playNextQueueLen(), queue().length);
    return index >= start && index < end;
  };

  return {
    status,
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
    gaplessStatus,
    volume,
    interruptLabel,
    isQueueIndexActive,
    queueIndexProgressPercent,
    isPlayNextQueueIndex,
    play,
    pause,
    togglePlayPause,
    prev,
    next,
    seek,
    previewSeek,
    setVolume,
    previewVolume: previewVolumeChange,
    commitPreviewedVolume,
    playQueueIndex,
    playAlbum,
    playFolderAlbum,
    playSongs,
    insertAfterCurrent,
    appendToQueue,
    clearQueue,
  };
}
