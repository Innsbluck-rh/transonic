import { Component, createEffect, createMemo, createSignal, onCleanup, Show } from 'solid-js';
import { commands, PlaybackState, SongResponse } from '~/bindings';
import { setPlaybackStateWithResult } from '~/features/playback/service';
import { playbackStore } from '~/stores/PlaybackStore';
import LoadCircle from '../common/LoadCircle';
import PlayerIcon from './PlayerIcon';
import PlayerSlider from './PlayerSlider';

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

const PlayerBar: Component = () => {
  const [state, setState] = createSignal<PlaybackState>();
  const [isLoading, setIsLoading] = createSignal<boolean>(false);
  const [isInactive, setIsInactive] = createSignal<boolean>(false);
  const [playingSong, setPlayingSong] = createSignal<SongResponse>();
  const [basePositionMs, setBasePositionMs] = createSignal<number>(0);
  const [positionSyncedAtMs, setPositionSyncedAtMs] = createSignal<number>(Date.now());
  const [nowMs, setNowMs] = createSignal<number>(Date.now());
  const [seekPreviewMs, setSeekPreviewMs] = createSignal<number | null>(null);

  const songDurationMs = createMemo(() => {
    const durationSeconds = playingSong()?.duration ?? 0;
    return durationSeconds * 1000;
  });

  const currentPositionMs = createMemo(() => {
    const previewPositionMs = seekPreviewMs();
    if (previewPositionMs !== null) {
      return previewPositionMs;
    }

    const status = playbackStore.status;
    if (!status) {
      return 0;
    }

    const syncedPositionMs = basePositionMs();
    if (status.state !== 'playing') {
      return Math.min(syncedPositionMs, songDurationMs() || syncedPositionMs);
    }

    const elapsedMs = Math.max(0, nowMs() - positionSyncedAtMs());
    const estimatedPositionMs = syncedPositionMs + elapsedMs;
    const maxPositionMs = songDurationMs();

    return maxPositionMs > 0 ? Math.min(estimatedPositionMs, maxPositionMs) : estimatedPositionMs;
  });

  const progressRatio = createMemo(() => {
    const durationMs = songDurationMs();
    if (durationMs <= 0) {
      return 0;
    }

    return currentPositionMs() / durationMs;
  });

  const canSeek = createMemo(() => {
    const durationMs = songDurationMs();
    return !isInactive() && !isLoading() && durationMs > 0;
  });

  const handleSeekPreview = (ratio: number | null) => {
    if (ratio === null) {
      setSeekPreviewMs(null);
      return;
    }

    setSeekPreviewMs(Math.round(songDurationMs() * ratio));
  };

  const handleSeekCommit = async (ratio: number) => {
    const durationMs = songDurationMs();
    if (durationMs <= 0) {
      setSeekPreviewMs(null);
      return;
    }

    const nextPositionMs = Math.round(durationMs * ratio);
    setSeekPreviewMs(nextPositionMs);

    const result = await commands.playbackSeek({ positionMs: nextPositionMs });
    await setPlaybackStateWithResult(result);
    setSeekPreviewMs(null);
  };

  createEffect(() => {
    const intervalId = window.setInterval(() => {
      setNowMs(Date.now());
    }, 500);

    onCleanup(() => window.clearInterval(intervalId));
  });

  createEffect(async () => {
    const status = playbackStore.status;
    if (status) {
      const state = status.state;
      setState(state);

      // TODO: confirm idle is really an inactive state
      setIsInactive(!state || state === 'error' || state === 'idle');
      setIsLoading(state === 'loading');
      setBasePositionMs(status.currentPositionMs);
      setPositionSyncedAtMs(Date.now());
      setNowMs(Date.now());

      if (status.currentSongId) {
        // get song info
        const songResult = await commands.getSong({ id: status.currentSongId });
        if (songResult.status === 'error') {
          console.error(songResult.error);
        } else {
          setPlayingSong(songResult.data);
        }
      } else {
        setPlayingSong(undefined);
      }
    }
  });

  return (
    <div class='flex flex-col relative w-full h-18 '>
      <div class='absolute w-full translate-y-[-50%]'>
        <PlayerSlider progressRatio={progressRatio()} disabled={!canSeek()} onSeekPreview={handleSeekPreview} onSeekCommit={handleSeekCommit} />
      </div>
      <div class='flex flex-row w-full h-full px-4 gap-2 items-center'>
        <div class='flex flex-row gap-2'>
          <PlayerIcon
            type='prev'
            disabled={isInactive()}
            onClick={async () => {
              const result = await commands.playbackPrev();
              await setPlaybackStateWithResult(result);
            }}
          />
          <Show when={!isLoading()} fallback={<LoadCircle />}>
            <PlayerIcon
              type={state() === 'playing' ? 'pause' : 'play'}
              disabled={isInactive()}
              onClick={async () => {
                if (state() === 'playing') {
                  const result = await commands.playbackPause();
                  await setPlaybackStateWithResult(result);
                } else {
                  const result = await commands.playbackPlay();
                  await setPlaybackStateWithResult(result);
                }
              }}
            />
          </Show>
          <PlayerIcon
            type='next'
            disabled={isInactive()}
            onClick={async () => {
              const result = await commands.playbackNext();
              await setPlaybackStateWithResult(result);
            }}
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

export default PlayerBar;
