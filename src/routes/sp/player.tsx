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

            <div class='flex flex-row w-full gap-10 px-12 items-center justify-evenly border-secondary-border mt-8'>
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
          <div class='flex flex-row w-full justify-between mt-10 px-2'>
            <p class='archivo text-xs font-bold text-accent'>{currentPositionText()}</p>
            <p class='archivo text-xs font-bold'>{durationText()}</p>
          </div>
        </div>

        <Show when={coverArt()}>
          {(dataUrl) => <img class='absolute w-full h-full object-cover object-center' src={dataUrl()} loading='lazy' decoding='async' />}
        </Show>
        <div class='absolute z-0 w-full h-full  backdrop-blur-[5px]' />
        <div class='absolute z-0 w-full h-full bg-primary-plane opacity-60' />

        <div class='absolute bottom-0 w-full translate-y-[50%] z-30'>
          <PlayerSlider valueMs={currentPositionMs()} maxMs={durationMs()} disabled={!canSeek()} onPreview={previewSeek} onCommit={seek} />
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
