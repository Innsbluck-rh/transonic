import { Component, createMemo, createResource, JSX, Match, Show, Switch } from 'solid-js';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import { fetchCoverArtAssetUrl } from '~/features/albums/service';
import { usePlayback } from '~/features/playback/usePlayback';
import { sessionStore } from '~/stores/SessionStore';
import PlayerIcon from './PlayerIcon';
import PlayerSlider from './PlayerSlider';

// this may include fast_prev and fast_next later
type PlayerIcons = 'prev' | 'playpause' | 'next';

interface PlayerBarProps {
  iconsVisibility?: Record<PlayerIcons, boolean>;
  onClickRestArea?: () => void;
  restAreaProps?: JSX.HTMLAttributes<HTMLDivElement>;
}

const PlayerBar: Component<PlayerBarProps> = (props) => {
  const iconsVisibility: Record<PlayerIcons, boolean> = props.iconsVisibility ?? {
    prev: true,
    playpause: true,
    next: true,
  };
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
    interruptLabel: playbackInterruptLabel,
    togglePlayPause,
    prev,
    next,
    seek,
    previewSeek,
  } = usePlayback();

  const subtitleText = createMemo(() => {
    const artist = currentEntry()?.artist || '[unknown]';
    return isInterrupted() ? `${artist} ${playbackInterruptLabel()}` : artist;
  });

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
  const restAreaClass = createMemo(() => props.restAreaProps?.class);

  const onClickRestArea: JSX.EventHandlerUnion<HTMLDivElement, MouseEvent> = (event) => {
    event.stopPropagation();
    props.onClickRestArea?.();
  };

  return (
    <div class='relative flex w-full flex-col shadow-[0_4px_10px_rgba(128,128,128,0.5)]'>
      <div class='absolute z-30 w-full translate-y-[-50%]'>
        <PlayerSlider valueMs={currentPositionMs()} maxMs={durationMs()} disabled={!canSeek()} onPreview={previewSeek} onCommit={seek} />
      </div>

      <Show when={coverArt()}>
        {(assetUrl) => <img class='absolute h-full w-full object-cover object-center' src={assetUrl()} loading='lazy' decoding='async' />}
      </Show>
      <div class='absolute z-0 h-full w-full backdrop-blur-[5px]'></div>
      <div class='bg-primary-plane absolute z-0 h-full w-full opacity-75'></div>
      <div class='from-primary-plane absolute z-0 h-full w-full bg-linear-to-r to-transparent'></div>

      <div class='relative z-10 flex h-full w-full flex-row items-center gap-2 p-4 shadow-xl'>
        <div class='z-10 flex flex-row items-center gap-2'>
          <Show when={iconsVisibility.prev}>
            <PlayerIcon type='prev' disabled={isControlDisabled()} onClick={prev} />
          </Show>
          <Show when={iconsVisibility.playpause}>
            <PlayerIcon
              type={playingState() === 'playing' ? 'pause' : 'play'}
              disabled={isControlDisabled()}
              loading={isInterrupted()}
              onClick={togglePlayPause}
            />
          </Show>
          <Show when={iconsVisibility.next}>
            <PlayerIcon type='next' disabled={isControlDisabled()} onClick={next} />
          </Show>
        </div>

        <div {...props.restAreaProps} class={`ml-2 flex min-w-0 flex-1 items-center ${restAreaClass() ?? ''}`} onClick={onClickRestArea}>
          <Switch
            fallback={
              <>
                <div class='mt-1 flex min-w-0 flex-1 flex-col gap-1'>
                  <MarqueeParagraph text={currentEntry()?.title || '[unknown]'} class='archivo leading-none font-bold' />
                  <MarqueeParagraph text={subtitleText()} class='archivo text-secondary-text text-xs leading-none' pixelsPerSecond={40} />
                </div>
                <div class='ml-4 flex flex-col gap-1'>
                  <p class='archivo text-xs'>
                    {currentPositionText()} / {durationText()}
                  </p>
                </div>
              </>
            }
          >
            <Match when={playingState() === 'error'}>
              <div class='flex-1'>
                <p class='archivo text-secondary-text italic'>[error occured]</p>
              </div>
            </Match>
            <Match when={playingState() === 'idle'}>
              <div class='flex-1'>
                <p class='archivo text-secondary-text italic'>[nothing played]</p>
              </div>
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  );
};

export default PlayerBar;
