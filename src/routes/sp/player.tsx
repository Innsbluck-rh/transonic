import { useNavigate } from '@solidjs/router';
import { createMemo, createResource, Show } from 'solid-js';
import Heading3 from '~/components/common/Heading3';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import PlayerIcon from '~/components/player/PlayerIcon';
import PlayerSlider from '~/components/player/PlayerSlider';
import QueueContent from '~/components/sidebar/queue/QueueContent';
import { fetchCoverArtDataUrl } from '~/features/albums/service';
import { usePlayback } from '~/features/playback/usePlayback';

function SPPlayer() {
  const {
    currentEntry,
    playingState,
    isInterrupted,
    isControlDisabled,
    canSeek,
    currentPositionMs,
    durationMs,
    currentPositionText,
    durationText,
    togglePlayPause,
    prev,
    next,
    seek,
    previewSeek,
  } = usePlayback();

  const DEFAULT_COVER_ART_SIZE = 224;
  const request = createMemo(() => {
    const coverArtId = currentEntry()?.coverArtId;
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
    <div class='flex min-h-0 flex-1 flex-col'>
      <div class='relative flex h-auto w-full flex-col items-center shadow-[0_0_16px_rgba(128,128,128,0.25)]'>
        <div class='z-10 mt-10 mb-2 flex h-full w-full flex-col items-center'>
          <div class='flex h-full w-full flex-col items-center px-8'>
            <Show when={coverArt()}>
              {(dataUrl) => (
                <img
                  class='mb-4 aspect-square h-auto w-1/2 object-cover object-center shadow-[0_0_24px_rgba(128,128,128,0.25)]'
                  src={dataUrl()}
                  loading='lazy'
                  decoding='async'
                />
              )}
            </Show>

            <MarqueeParagraph text={currentEntry()?.title || ''} class='archivo text-xl font-bold' />
            <MarqueeParagraph
              text={currentEntry()?.artist || ''}
              class='archivo'
              onClick={() => {
                const artistId = currentEntry()?.artistId;
                if (!artistId) return;
                navigate(`/browse/artists/${artistId}`);
              }}
            />

            <div class='border-secondary-border mt-8 flex w-full flex-row items-center justify-evenly gap-10 px-12'>
              <PlayerIcon iconClass='scale-250' type='prev' disabled={isControlDisabled()} onClick={prev} />
              <PlayerIcon
                iconClass='scale-350'
                type={playingState() === 'playing' ? 'pause' : 'play'}
                disabled={isControlDisabled()}
                loading={isInterrupted()}
                onClick={togglePlayPause}
              />
              <PlayerIcon iconClass='scale-250' type='next' disabled={isControlDisabled()} onClick={next} />
            </div>
          </div>
          <div class='mt-10 flex w-full flex-row justify-between px-2'>
            <p class='archivo text-accent text-xs font-bold'>{currentPositionText()}</p>
            <p class='archivo text-xs font-bold'>{durationText()}</p>
          </div>
        </div>

        <Show when={coverArt()}>
          {(dataUrl) => <img class='absolute h-full w-full object-cover object-center' src={dataUrl()} loading='lazy' decoding='async' />}
        </Show>
        <div class='absolute z-0 h-full w-full backdrop-blur-[5px]' />
        <div class='bg-primary-plane absolute z-0 h-full w-full opacity-60' />

        <div class='absolute bottom-0 z-30 w-full translate-y-[50%]'>
          <PlayerSlider valueMs={currentPositionMs()} maxMs={durationMs()} disabled={!canSeek()} onPreview={previewSeek} onCommit={seek} />
        </div>
      </div>

      <div class='bg-primary-plane border-secondary-border flex flex-row items-center border-b p-2'>
        <Heading3>queue</Heading3>
      </div>
      <QueueContent />
    </div>
  );
}

export default SPPlayer;
