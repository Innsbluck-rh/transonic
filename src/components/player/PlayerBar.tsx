import { Component, createEffect, createMemo, createResource, createSignal, Match, Show, Switch } from 'solid-js';
import { commands, InterruptReason, PlayingState, SongResponse } from '~/bindings';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import { fetchCoverArtDataUrl } from '~/features/albums/service';
import { hasPlaybackCommandError } from '~/features/playback/service';
import { playbackStore } from '~/stores/PlaybackStore';
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

function isControlDisabled(state?: PlayingState) {
  return !state || state === 'idle' || state === 'error' || state === 'interrupted';
}

function isSeekDisabled(state?: PlayingState) {
  return !state || state === 'idle' || state === 'error' || state === 'interrupted';
}

function clampPositionMs(positionMs: number, durationMs: number) {
  if (durationMs <= 0) {
    return Math.max(0, positionMs);
  }

  return Math.min(Math.max(0, positionMs), durationMs);
}

// this may include fast_prev and fast_next later
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
  const [previewPositionMs, setPreviewPositionMs] = createSignal<number | null>(null);

  const status = createMemo(() => playbackStore.status);
  const playingState = createMemo(() => status()?.playingState);
  const interruptReason = createMemo<InterruptReason | null>(() => status()?.interruptReason ?? null);
  const pendingSeekPositionMs = createMemo(() => status()?.pendingSeekPositionMs ?? null);
  const isInterrupted = createMemo(() => playingState() === 'interrupted');
  const isDisabled = createMemo(() => isControlDisabled(playingState()));
  const currentSongId = createMemo(() => status()?.currentSongId ?? null);
  const interruptLabel = createMemo(() => {
    switch (interruptReason()) {
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
  });
  const subtitleText = createMemo(() => {
    const artist = playingSong()?.artist || '[unknown]';
    return isInterrupted() ? `${artist} ${interruptLabel()}` : artist;
  });

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

    return clampPositionMs(status()?.currentPositionMs ?? 0, songDurationMs());
  });

  const canSeek = createMemo(() => {
    return !isSeekDisabled(playingState()) && songDurationMs() > 0;
  });

  const handleSeekCommit = async (nextPositionMs: number) => {
    const durationMs = songDurationMs();
    if (durationMs <= 0) {
      setPreviewPositionMs(null);
      return;
    }

    const clampedPositionMs = clampPositionMs(nextPositionMs, durationMs);
    setPreviewPositionMs(null);
    const result = await commands.playbackSeek({ positionMs: clampedPositionMs });
    hasPlaybackCommandError(result);
  };

  const handlePrev = async () => {
    const result = await commands.playbackPrev();
    hasPlaybackCommandError(result);
  };

  const handlePlayPause = async () => {
    if (playingState() === 'playing') {
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
          disabled={!canSeek()}
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
            <PlayerIcon type='prev' disabled={isDisabled()} onClick={handlePrev} />
          </Show>
          <Show when={iconsVisibility.playpause}>
            <PlayerIcon
              type={playingState() === 'playing' ? 'pause' : 'play'}
              disabled={isDisabled()}
              loading={isInterrupted()}
              onClick={handlePlayPause}
            />
          </Show>
          <Show when={iconsVisibility.next}>
            <PlayerIcon type='next' disabled={isDisabled()} onClick={handleNext} />
          </Show>
        </div>

        <Switch
          fallback={
            <>
              <div class='flex min-w-0 flex-col flex-1 ml-2'>
                <MarqueeParagraph text={playingSong()?.title || '[unknown]'} class='archivo font-bold' />
                <MarqueeParagraph text={subtitleText()} class='archivo italic text-xs leading-none text-secondary-text' pixelsPerSecond={40} />
              </div>
              <div class='flex flex-col gap-1 mr-1'>
                <p class='archivo text-xs'>
                  {formatTime(currentPositionMs() / 1000)} / {formatTime(songDurationMs() / 1000)}
                </p>
              </div>
            </>
          }
        >
          <Match when={playingState() === 'error'}>
            <div class='flex-1 ml-2'>
              <p class='archivo italic text-secondary-text'>[error occured]</p>
            </div>
          </Match>
          <Match when={playingState() === 'idle'}>
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
