import { Component, JSX, Show } from 'solid-js';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import PlayerIcon from '~/components/player/PlayerIcon';
import PlayerSlider from '~/components/player/PlayerSlider';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { resolveAlbumRoute, resolveArtistRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { usePlayback } from '~/features/playback/usePlayback';
import { buildQueueAutoScrollKey, useQueueAutoScroll } from '~/features/playback/useQueueAutoScroll';
import Heading3 from '../common/Heading3';
import QueueList from '../common/list/song/QueueList';
import LoadCircle from '../common/LoadCircle';

interface SPPlayerProps {
  topSectionProps?: JSX.HTMLAttributes<HTMLDivElement>;
}

const SPPlayer: Component<SPPlayerProps> = (props) => {
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
    queue,
    currentIndex,
  } = usePlayback();

  const { src: coverArt } = useCoverArt(() => currentEntry()?.coverArtId, CoverArtSizes.lg);

  const navigate = useSPNavigate();
  const { setScrollContainerRef, setItemRef } = useQueueAutoScroll({
    currentIndex,
    currentEntryId: () => currentEntry()?.id ?? null,
    queueKey: () => buildQueueAutoScrollKey(queue()),
    defaultBehavior: 'smooth',
    initialBehavior: 'instant',
    queueChangeBehavior: 'instant',
  });

  return (
    <div class='z-50 flex h-full min-h-0 flex-1 flex-col'>
      <div {...props.topSectionProps} class={`relative flex h-auto w-full shrink-0 flex-col items-center ${props.topSectionProps?.class ?? ''}`}>
        <div class='z-10 mt-4 flex size-full flex-col items-center'>
          <div class='flex size-full flex-col items-center px-8'>
            <div class='relative flex aspect-square h-auto w-46 min-w-1/3 shadow-xl'>
              <Show
                when={coverArt()}
                fallback={
                  <div class='border-secondary-border absolute inset-0 flex items-center justify-center border'>
                    <LoadCircle />
                  </div>
                }
              >
                {(assetUrl) => <img class='absolute inset-0 object-cover object-center' src={assetUrl()} loading='lazy' decoding='async' />}
              </Show>
            </div>
            <MarqueeParagraph
              text={currentEntry()?.title || ''}
              class='archivo mt-4 w-full text-center text-2xl font-black tracking-tighter'
              onPointerDown={(event) => {
                event.stopPropagation();
              }}
              onClick={(event) => {
                event.stopPropagation();
                const albumId = currentEntry()?.albumId;
                if (!albumId) return;
                navigate(resolveAlbumRoute(albumId));
              }}
            />
            <MarqueeParagraph
              text={currentEntry()?.artist || ''}
              class='archivo text-secondary-text mt-1'
              onPointerDown={(event) => {
                event.stopPropagation();
              }}
              onClick={(event) => {
                event.stopPropagation();
                const artistId = currentEntry()?.artistId;
                if (!artistId) return;
                navigate(resolveArtistRoute(artistId));
              }}
            />

            <div
              class='border-secondary-border mt-2 mb-4 flex w-full flex-row items-center justify-evenly'
              onPointerDown={(event) => {
                event.stopPropagation();
              }}
            >
              <PlayerIcon iconClass='scale-200 p-6' type='prev' disabled={isControlDisabled()} onClick={prev} />
              <PlayerIcon
                iconClass='scale-350 p-5'
                type={playingState() === 'playing' ? 'pause' : 'play'}
                disabled={isControlDisabled()}
                loading={isInterrupted()}
                onClick={togglePlayPause}
              />
              <PlayerIcon iconClass='scale-200 p-6' type='next' disabled={isControlDisabled()} onClick={next} />
            </div>
          </div>
          <div class='absolute bottom-0 mb-2 flex w-full flex-row justify-between px-2'>
            <p class='archivo text-accent text-xs font-bold'>{currentPositionText()}</p>
            <p class='archivo text-xs font-bold'>{durationText()}</p>
          </div>
        </div>

        <div class='bg-primary-plane absolute inset-0 -z-10 size-full overflow-hidden'>
          <Show when={coverArt()}>
            {(assetUrl) => <img class='absolute -z-10 size-full object-cover object-center' src={assetUrl()} loading='lazy' decoding='async' />}
          </Show>
          <div class='absolute -z-10 size-full backdrop-blur-[5px]' />
          <div class='bg-primary-plane absolute -z-10 size-full opacity-75' />
        </div>

        <div class='absolute bottom-0 z-30 w-full translate-y-[50%]'>
          <PlayerSlider valueMs={currentPositionMs()} maxMs={durationMs()} disabled={!canSeek()} onPreview={previewSeek} onCommit={seek} />
        </div>
      </div>

      <div class='border-secondary-border flex flex-row items-center border-b px-3 pt-2 pb-1.5'>
        <Heading3 class='text-secondary-text'>queue</Heading3>
      </div>
      <div ref={setScrollContainerRef} class='flex-1 overflow-y-auto pb-3'>
        <QueueList queue={queue()} itemRef={setItemRef} />
      </div>
    </div>
  );
};

export default SPPlayer;
