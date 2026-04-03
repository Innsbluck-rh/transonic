import { useNavigate } from '@solidjs/router';
import { Component, createMemo, createResource, JSX, Show } from 'solid-js';
import Heading3 from '~/components/common/Heading3';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import PlayerIcon from '~/components/player/PlayerIcon';
import PlayerSlider from '~/components/player/PlayerSlider';
import QueueContent from '~/components/sidebar/queue/QueueContent';
import { fetchCoverArtAssetUrl } from '~/features/albums/service';
import { resolveArtistRoute } from '~/features/navigation/routes';
import { usePlayback } from '~/features/playback/usePlayback';
import { sessionStore } from '~/stores/SessionStore';

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

  const DEFAULT_COVER_ART_SIZE = 224;
  const request = createMemo(() => {
    const coverArtId = currentEntry()?.coverArtId;
    const profileId = sessionStore.activeSession?.profileId;
    if (!coverArtId || !profileId) {
      return null;
    }

    return {
      profileId,
      coverArtId,
      size: DEFAULT_COVER_ART_SIZE,
    };
  });

  const [coverArt] = createResource(request, (payload) => fetchCoverArtAssetUrl(payload));

  const navigate = useNavigate();

  return (
    <div class='flex h-full min-h-0 flex-1 flex-col'>
      <div
        {...props.topSectionProps}
        class={`relative flex h-auto w-full shrink-0 flex-col items-center shadow-[0_0_16px_rgba(128,128,128,0.25)] ${props.topSectionProps?.class ?? ''}`}
      >
        <div class='z-10 mt-14 mb-2 flex h-full w-full flex-col items-center'>
          <div class='flex h-full w-full flex-col items-center px-8'>
            <Show when={coverArt()}>
              {(assetUrl) => (
                <img
                  class='mb-6 aspect-square h-auto w-90 max-w-[70%] min-w-1/2 object-cover object-center shadow-[0_0_24px_rgba(128,128,128,0.25)]'
                  src={assetUrl()}
                  loading='lazy'
                  decoding='async'
                />
              )}
            </Show>

            <MarqueeParagraph
              text={currentEntry()?.title || ''}
              class='archivo-black w-full text-center text-2xl font-black tracking-tighter italic'
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
                navigate(resolveArtistRoute(artistId, 'mobile'));
              }}
            />

            <div
              class='border-secondary-border mt-8 flex w-full flex-row items-center justify-evenly gap-10 px-12'
              onPointerDown={(event) => {
                event.stopPropagation();
              }}
            >
              <PlayerIcon iconClass='scale-200' type='prev' disabled={isControlDisabled()} onClick={prev} />
              <PlayerIcon
                iconClass='scale-350'
                type={playingState() === 'playing' ? 'pause' : 'play'}
                disabled={isControlDisabled()}
                loading={isInterrupted()}
                onClick={togglePlayPause}
              />
              <PlayerIcon iconClass='scale-200' type='next' disabled={isControlDisabled()} onClick={next} />
            </div>
          </div>
          <div class='mt-10 flex w-full flex-row justify-between px-2'>
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

      <div class='bg-primary-plane border-secondary-border flex flex-row items-center border-b p-2'>
        <Heading3>queue</Heading3>
      </div>
      <QueueContent />
    </div>
  );
};

export default SPPlayer;
