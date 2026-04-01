import { Component, createEffect, createMemo, createSignal, onCleanup } from 'solid-js';
import { commands, PlaybackState, SongResponse } from '~/bindings';
import { hasPlaybackCommandError } from '~/features/playback/service';
import { playbackStore } from '~/stores/PlaybackStore';
import PlayerIcon from './PlayerIcon';
import PlayerSlider from './PlayerSlider';

const PLAYBACK_PROGRESS_TICK_MS = 500;

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

function isInactivePlaybackState(state?: PlaybackState) {
  return !state || state === 'loading' || state === 'error' || state === 'idle';
}

function clampPositionMs(positionMs: number, durationMs: number) {
  if (durationMs <= 0) {
    return Math.max(0, positionMs);
  }

  return Math.min(Math.max(0, positionMs), durationMs);
}

const PlayerBarSP: Component = () => {
  const [playingSong, setPlayingSong] = createSignal<SongResponse | null>(null);
  const [syncedPositionMs, setSyncedPositionMs] = createSignal<number>(0);
  const [positionSyncedAtMs, setPositionSyncedAtMs] = createSignal<number>(Date.now());
  const [nowMs, setNowMs] = createSignal<number>(Date.now());
  const [previewPositionMs, setPreviewPositionMs] = createSignal<number | null>(null);

  const status = createMemo(() => playbackStore.status);
  const playbackState = createMemo(() => status()?.state);
  const isLoading = createMemo(() => playbackState() === 'loading');
  const isInactive = createMemo(() => isInactivePlaybackState(playbackState()));
  const currentSongId = createMemo(() => status()?.currentSongId ?? null);

  const syncPlaybackPosition = (positionMs: number) => {
    const syncedAtMs = Date.now();
    setSyncedPositionMs(positionMs);
    setPositionSyncedAtMs(syncedAtMs);
    setNowMs(syncedAtMs);
    setPreviewPositionMs(null);
  };

  const songDurationMs = createMemo(() => {
    const durationSeconds = playingSong()?.duration ?? 0;
    return durationSeconds * 1000;
  });

  const currentPositionMs = createMemo(() => {
    const nextPreviewPositionMs = previewPositionMs();
    if (nextPreviewPositionMs !== null) {
      return clampPositionMs(nextPreviewPositionMs, songDurationMs());
    }

    const currentStatus = status();
    if (!currentStatus) {
      return 0;
    }

    const durationMs = songDurationMs();
    const basePositionMs = syncedPositionMs();
    if (currentStatus.state !== 'playing') {
      return clampPositionMs(basePositionMs, durationMs);
    }

    const elapsedMs = Math.max(0, nowMs() - positionSyncedAtMs());
    return clampPositionMs(basePositionMs + elapsedMs, durationMs);
  });

  const canSeek = createMemo(() => {
    return !isInactive() && songDurationMs() > 0;
  });

  const handleSeekCommit = async (nextPositionMs: number) => {
    const durationMs = songDurationMs();
    if (durationMs <= 0) {
      setPreviewPositionMs(null);
      return;
    }

    setPreviewPositionMs(nextPositionMs);

    const result = await commands.playbackSeek({ positionMs: nextPositionMs });
    if (hasPlaybackCommandError(result)) {
      setPreviewPositionMs(null);
    }
  };

  const handlePrev = async () => {
    const result = await commands.playbackPrev();
    hasPlaybackCommandError(result);
  };

  const handlePlayPause = async () => {
    if (playbackState() === 'playing') {
      const result = await commands.playbackPause();
      hasPlaybackCommandError(result);
      return;
    }

    const result = await commands.playbackPlay();
    hasPlaybackCommandError(result);
  };

  const handleNext = async () => {
    const result = await commands.playbackNext();
    hasPlaybackCommandError(result);
  };

  createEffect(() => {
    const intervalId = window.setInterval(() => {
      setNowMs(Date.now());
    }, PLAYBACK_PROGRESS_TICK_MS);

    onCleanup(() => window.clearInterval(intervalId));
  });

  createEffect(() => {
    const currentStatus = status();
    if (!currentStatus) {
      syncPlaybackPosition(0);
      return;
    }

    syncPlaybackPosition(currentStatus.currentPositionMs);
  });

  createEffect(() => {
    const songId = currentSongId();
    if (!songId) {
      setPlayingSong(null);
      return;
    }

    void (async () => {
      const songResult = await commands.getSong({ id: songId });
      if (currentSongId() !== songId) {
        return;
      }

      if (songResult.status === 'error') {
        console.error(songResult.error);
        setPlayingSong(null);
        return;
      }

      setPlayingSong(songResult.data);
    })();
  });

  return (
    <div class='flex flex-col relative w-full h-18 '>
      <div class='absolute w-full translate-y-[-50%]'>
        <PlayerSlider
          valueMs={currentPositionMs()}
          maxMs={songDurationMs()}
          disabled={!canSeek()}
          onPreview={(valueMs) => setPreviewPositionMs(valueMs)}
          onCommit={handleSeekCommit}
        />
      </div>
      <div class='flex flex-row w-full h-full px-4 gap-2 items-center'>
        <div class='flex flex-row gap-2 items-center'>
          <PlayerIcon
            type={playbackState() === 'playing' ? 'pause' : 'play'}
            disabled={isInactive()}
            loading={isLoading()}
            onClick={handlePlayPause}
          />
        </div>

        <div class='flex flex-col flex-1 ml-2'>
          <p class='archivo bold'>{playingSong()?.title || 'play something...'}</p>
          <p class='archivo italic text-xs leading-none opacity-75'>{playingSong()?.artist || '[unknown]'}</p>
        </div>
        <div class='flex flex-col gap-1'>
          <p class='archivo text-xs'>
            {formatTime(currentPositionMs() / 1000)} / {formatTime(songDurationMs() / 1000)}
          </p>
        </div>
      </div>
    </div>
  );
};

export default PlayerBarSP;
