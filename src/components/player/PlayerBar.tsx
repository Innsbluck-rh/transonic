import { Component, createEffect, createMemo, createResource, createSignal, Match, onCleanup, Show, Switch } from 'solid-js';
import { commands, PlaybackLoadingReason, PlaybackState, SongResponse } from '~/bindings';
import { fetchCoverArtDataUrl } from '~/features/albums/service';
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

// this may include
type PlayerIcons = 'prev' | 'playpause' | 'next';

interface PlayerBarProps {
  iconsVisibility?: Record<PlayerIcons, boolean>;
}

const PlayerBar: Component<PlayerBarProps> = (props) => {
  const iconsVisibility: Record<PlayerIcons, boolean> = props.iconsVisibility ?? {
    prev: true,
    playpause: true,
    next: true,
  };

  const [playingSong, setPlayingSong] = createSignal<SongResponse | null>(null);
  const [syncedPositionMs, setSyncedPositionMs] = createSignal<number>(0);
  const [positionSyncedAtMs, setPositionSyncedAtMs] = createSignal<number>(Date.now());
  const [nowMs, setNowMs] = createSignal<number>(Date.now());
  const [previewPositionMs, setPreviewPositionMs] = createSignal<number | null>(null);
  const [pendingSeekPositionMs, setPendingSeekPositionMs] = createSignal<number | null>(null);
  const [seeking, setSeeking] = createSignal<boolean>(false);
  const [canAnimatePlayingProgress, setCanAnimatePlayingProgress] = createSignal<boolean>(false);

  const status = createMemo(() => playbackStore.status);
  const playbackState = createMemo(() => status()?.state);
  const loadingReason = createMemo<PlaybackLoadingReason | null>(() => status()?.loadingReason ?? null);
  const isLoading = createMemo(() => playbackState() === 'loading');
  const isInactive = createMemo(() => isInactivePlaybackState(playbackState()));
  const currentSongId = createMemo(() => status()?.currentSongId ?? null);
  const loadingLabel = createMemo(() => {
    switch (loadingReason()) {
      case 'seeking':
        return '[seeking...]';
      case 'buffering':
        return '[buffering...]';
      default:
        return '[loading...]';
    }
  });
  const subtitleText = createMemo(() => {
    const artist = playingSong()?.artist || '[unknown]';
    return isLoading() ? `${artist} ${loadingLabel()}` : artist;
  });

  const syncPlaybackPosition = (positionMs: number) => {
    const syncedAtMs = Date.now();
    setSyncedPositionMs(positionMs);
    setPositionSyncedAtMs(syncedAtMs);
    setNowMs(syncedAtMs);
    setPreviewPositionMs(null);
    setPendingSeekPositionMs(null);
    setSeeking(false);
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

    const nextPendingSeekPositionMs = pendingSeekPositionMs();
    if (nextPendingSeekPositionMs !== null) {
      return clampPositionMs(nextPendingSeekPositionMs, songDurationMs());
    }

    const currentStatus = status();
    if (!currentStatus) {
      return 0;
    }

    const durationMs = songDurationMs();
    const basePositionMs = syncedPositionMs();
    if (currentStatus.state !== 'playing' || !canAnimatePlayingProgress()) {
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
    if (durationMs <= 0 || seeking()) {
      setPreviewPositionMs(null);
      return;
    }

    const clampedPositionMs = clampPositionMs(nextPositionMs, durationMs);
    setPreviewPositionMs(null);
    setPendingSeekPositionMs(clampedPositionMs);
    setSeeking(true);

    const result = await commands.playbackSeek({ positionMs: clampedPositionMs });
    if (hasPlaybackCommandError(result)) {
      setPendingSeekPositionMs(null);
      setSeeking(false);
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

  createEffect((previousPlaybackState?: PlaybackState) => {
    const currentStatus = status();
    if (!currentStatus) {
      setCanAnimatePlayingProgress(false);
      syncPlaybackPosition(0);
      return undefined;
    }

    syncPlaybackPosition(currentStatus.currentPositionMs);
    const confirmedPlayingUpdate = currentStatus.state === 'playing' && (previousPlaybackState === 'playing' || currentStatus.currentPositionMs > 0);
    setCanAnimatePlayingProgress(confirmedPlayingUpdate);
    return currentStatus.state;
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

  const DEFAULT_COVER_ART_SIZE = 224;
  const request = createMemo(() => {
    const coverArtId = playingSong()?.coverArtId;
    if (!coverArtId) {
      return null;
    }

    return {
      coverArtId,
      size: DEFAULT_COVER_ART_SIZE,
    };
  });
  const [coverArt] = createResource(request, (payload) => fetchCoverArtDataUrl(payload));

  return (
    <div class='flex flex-col relative w-full h-18 shadow-[0_4px_10px_rgba(128,128,128,0.5)]'>
      <div class='absolute w-full translate-y-[-50%] z-30'>
        <PlayerSlider
          valueMs={currentPositionMs()}
          maxMs={songDurationMs()}
          disabled={!canSeek() || seeking()}
          onPreview={(valueMs) => setPreviewPositionMs(valueMs)}
          onCommit={handleSeekCommit}
        />
      </div>

      <Show when={coverArt()}>
        {(dataUrl) => <img class='absolute w-full h-full object-cover object-center' src={dataUrl()} loading='lazy' decoding='async' />}
      </Show>
      <div class='absolute z-0 w-full h-full  backdrop-blur-xs'></div>
      <div class='absolute z-0 w-full h-full bg-primary-plane opacity-75'></div>
      <div class='absolute z-0 w-full h-full  from-primary-plane to-transparent bg-linear-to-r'></div>

      <div class='flex flex-row w-full h-full px-4 gap-2 items-center shadow-xl z-10'>
        <div class='flex flex-row gap-2 items-center'>
          <Show when={iconsVisibility.prev}>
            <PlayerIcon type='prev' disabled={isInactive()} onClick={handlePrev} />
          </Show>
          <Show when={iconsVisibility.playpause}>
            <PlayerIcon
              type={playbackState() === 'playing' ? 'pause' : 'play'}
              disabled={isInactive()}
              loading={isLoading()}
              onClick={handlePlayPause}
            />
          </Show>
          <Show when={iconsVisibility.next}>
            <PlayerIcon type='next' disabled={isInactive()} onClick={handleNext} />
          </Show>
        </div>

        <Switch
          fallback={
            <>
              <div class='flex flex-col flex-1 ml-2'>
                <p class='archivo bold'>{playingSong()?.title || '[unknown]'}</p>
                <p class='archivo italic text-xs leading-none text-secondary-text'>{subtitleText()}</p>
              </div>
              <div class='flex flex-col gap-1 mr-1'>
                <p class='archivo text-xs'>
                  {formatTime(currentPositionMs() / 1000)} / {formatTime(songDurationMs() / 1000)}
                </p>
              </div>
            </>
          }
        >
          <Match when={playbackState() === 'error'}>
            <div class='flex-1 ml-2'>
              <p class='archivo italic text-secondary-text'>[error occured]</p>
            </div>
          </Match>
          <Match when={playbackState() === 'idle'}>
            <div class='flex-1 ml-2'>
              <p class='archivo italic text-secondary-text'>[nothing played]</p>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default PlayerBar;
