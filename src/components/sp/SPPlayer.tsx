import { Component, JSX, Show } from 'solid-js';
import Heading3 from '~/components/common/Heading3';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import PlayerIcon from '~/components/player/PlayerIcon';
import PlayerSlider from '~/components/player/PlayerSlider';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { resolveAlbumRoute, resolveArtistRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { usePlayback } from '~/features/playback/usePlayback';
import QueueList from '../common/list/song/QueueList';
import { closePlayerBar } from './SPExpandablePlayerBar';

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
  } = usePlayback();

  const { src: coverArt } = useCoverArt(() => currentEntry()?.coverArtId);

  const navigate = useSPNavigate();
  const { queue, isQueueIndexActive, playQueueIndex } = usePlayback();

  return (
    <div class='z-50 flex h-full min-h-0 flex-1 flex-col'>
      <div
        {...props.topSectionProps}
        class={`relative flex h-auto w-full shrink-0 flex-col items-center shadow-lg ${props.topSectionProps?.class ?? ''}`}
      >
        <div class='z-10 mt-7 flex h-full w-full flex-col items-center'>
          <div class='flex h-full w-full flex-col items-center px-8'>
            <Show when={coverArt()}>
              {(assetUrl) => (
                <img
                  class='aspect-square h-auto w-48 max-w-[70%] min-w-1/3 object-cover object-center shadow-xl'
                  src={assetUrl()}
                  loading='lazy'
                  decoding='async'
                />
              )}
            </Show>

            <MarqueeParagraph
              text={currentEntry()?.title || ''}
              class='archivo mt-5 w-full text-center text-2xl font-black tracking-tighter'
              onClick={() => {
                const albumId = currentEntry()?.albumId;
                if (!albumId) return;
                closePlayerBar(false);
                navigate(resolveAlbumRoute(albumId));
              }}
            />
            <MarqueeParagraph
              text={currentEntry()?.artist || ''}
              class='archivo text-secondary-text mt-1'
              onPointerDown={(event) => {
                event.stopPropagation();
              }}
              onClick={() => {
                const artistId = currentEntry()?.artistId;
                if (!artistId) return;
                closePlayerBar(false);
                navigate(resolveArtistRoute(artistId));
              }}
            />

            <div
              class='border-secondary-border mt-2 flex w-full flex-row items-center justify-evenly'
              onPointerDown={(event) => {
                event.stopPropagation();
              }}
            >
              <PlayerIcon iconClass='scale-200 p-6' type='prev' disabled={isControlDisabled()} onClick={prev} />
              <PlayerIcon
                iconClass='scale-350 p-6'
                type={playingState() === 'playing' ? 'pause' : 'play'}
                disabled={isControlDisabled()}
                loading={isInterrupted()}
                onClick={togglePlayPause}
              />
              <PlayerIcon iconClass='scale-200 p-6' type='next' disabled={isControlDisabled()} onClick={next} />
            </div>
          </div>
          <div class='mb-2 flex w-full flex-row justify-between px-2'>
            <p class='archivo text-accent text-xs font-bold'>{currentPositionText()}</p>
            <p class='archivo text-xs font-bold'>{durationText()}</p>
          </div>
        </div>

        <Show when={coverArt()}>
          {(assetUrl) => <img class='absolute h-full w-full object-cover object-center' src={assetUrl()} loading='lazy' decoding='async' />}
        </Show>
        <div class='absolute z-0 h-full w-full backdrop-blur-[5px]' />
        <div class='bg-primary-plane absolute z-0 h-full w-full opacity-75' />

        <div class='absolute bottom-0 z-30 w-full translate-y-[50%]'>
          <PlayerSlider valueMs={currentPositionMs()} maxMs={durationMs()} disabled={!canSeek()} onPreview={previewSeek} onCommit={seek} />
        </div>
      </div>

      <div class='bg-primary-plane border-secondary-border secondar flex flex-row items-center border-b px-3 pt-3.5 pb-1.5'>
        <Heading3 class='text-secondary-text'>queue</Heading3>
      </div>
      <div class='flex-1 overflow-y-auto'>
        <QueueList queue={queue()} />
      </div>
    </div>
  );
};

export default SPPlayer;
