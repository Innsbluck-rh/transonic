import { useNavigate } from '@solidjs/router';
import { createEffect, createMemo, createResource, createSignal, Show } from 'solid-js';
import { commands, InterruptReason, PlayingState, SongResponse } from '~/bindings';
import Heading3 from '~/components/common/Heading3';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import PlayerIcon from '~/components/player/PlayerIcon';
import PlayerSlider from '~/components/player/PlayerSlider';
import QueueContent from '~/components/sidebar/queue/QueueContent';
import { fetchCoverArtDataUrl } from '~/features/albums/service';
import { hasPlaybackCommandError } from '~/features/playback/service';
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
function SPPlayer() {
  const [playingSong, setPlayingSong] = createSignal<SongResponse | null>(null);

  const status = createMemo(() => playbackStore.status);
  const playingState = createMemo(() => status()?.playingState);
  const interruptReason = createMemo<InterruptReason | null>(() => status()?.interruptReason ?? null);
  const pendingSeekPositionMs = createMemo(() => status()?.pendingSeekPositionMs ?? null);
  const isInterrupted = createMemo(() => playingState() === 'interrupted');
  const isDisabled = createMemo(() => isControlDisabled(playingState()));
  const currentSongId = createMemo(() => status()?.currentSongId ?? null);

  function isControlDisabled(state?: PlayingState) {
    return !state || state === 'idle' || state === 'error' || state === 'interrupted';
  }
  const songDurationMs = createMemo(() => {
    const durationSeconds = playingSong()?.duration ?? 0;
    return durationSeconds * 1000;
  });

  function clampPositionMs(positionMs: number, durationMs: number) {
    if (durationMs <= 0) {
      return Math.max(0, positionMs);
    }

    return Math.min(Math.max(0, positionMs), durationMs);
  }

  const [previewPositionMs, setPreviewPositionMs] = createSignal<number | null>(null);
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

  function isSeekDisabled(state?: PlayingState) {
    return !state || state === 'idle' || state === 'error' || state === 'interrupted';
  }

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

  createEffect(() => {
    const songId = playbackStore.status?.currentSongId;
    if (!songId) {
      setPlayingSong(null);
      return;
    }

    void (async () => {
      const songResult = await commands.getSong({ id: songId });
      if (songResult.status === 'error') {
        console.error(songResult.error);
        setPlayingSong(null);
        return;
      }

      setPlayingSong(songResult.data);
    })();
  });

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

  const navigate = useNavigate();

  return (
    <div class='flex flex-col flex-1 min-h-0'>
      <div class='relative flex flex-col w-full h-auto items-center shadow-[0_0_16px_rgba(128,128,128,0.25)]'>
        <div class='flex flex-col w-full h-full items-center z-10 mt-10 mb-2'>
          <div class='flex flex-col w-full h-full items-center px-8'>
            <Show when={coverArt()}>
              {(dataUrl) => (
                <img
                  class='w-1/2 h-auto shadow-[0_0_24px_rgba(128,128,128,0.25)] aspect-square object-cover object-center mb-4'
                  src={dataUrl()}
                  loading='lazy'
                  decoding='async'
                />
              )}
            </Show>

            <MarqueeParagraph text={playingSong()?.title || ''} class='archivo text-xl font-bold' />
            <MarqueeParagraph
              text={playingSong()?.artist || ''}
              class='archivo'
              onClick={() => {
                navigate(`/browse/artists/${playingSong()?.artistId}`);
              }}
            />

            <div class='flex flex-row w-full gap-10 px-12 items-center justify-evenly border-secondary-border mt-8'>
              <PlayerIcon iconClass='scale-250' type='prev' disabled={isDisabled()} onClick={handlePrev} />
              <PlayerIcon
                iconClass='scale-350'
                type={playingState() === 'playing' ? 'pause' : 'play'}
                disabled={isDisabled()}
                loading={isInterrupted()}
                onClick={handlePlayPause}
              />
              <PlayerIcon iconClass='scale-250' type='next' disabled={isDisabled()} onClick={handleNext} />
            </div>
          </div>
          <div class='flex flex-row w-full justify-between mt-10 px-2'>
            <p class='archivo text-xs font-bold text-accent'>{formatTime(currentPositionMs() / 1000)}</p>
            <p class='archivo text-xs font-bold'>{formatTime(songDurationMs() / 1000)}</p>
          </div>
        </div>

        <Show when={coverArt()}>
          {(dataUrl) => <img class='absolute w-full h-full object-cover object-center' src={dataUrl()} loading='lazy' decoding='async' />}
        </Show>
        <div class='absolute z-0 w-full h-full  backdrop-blur-[5px]' />
        <div class='absolute z-0 w-full h-full bg-primary-plane opacity-60' />

        <div class='absolute bottom-0 w-full translate-y-[50%] z-30'>
          <PlayerSlider
            valueMs={currentPositionMs()}
            maxMs={songDurationMs()}
            disabled={!canSeek()}
            onPreview={(valueMs) => setPreviewPositionMs(valueMs)}
            onCommit={handleSeekCommit}
          />
        </div>
      </div>

      <div class='flex flex-row items-center bg-primary-plane border-b border-secondary-border p-2'>
        <Heading3>queue</Heading3>
      </div>
      <QueueContent />
    </div>
  );
}

export default SPPlayer;
