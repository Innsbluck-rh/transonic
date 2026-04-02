import { Component, createMemo, createResource, Match, Show, Switch } from 'solid-js';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import { fetchCoverArtDataUrl } from '~/features/albums/service';
import { usePlayback } from '~/features/playback/usePlayback';
import PlayerIcon from './PlayerIcon';
import PlayerSlider from './PlayerSlider';

// this may include fast_prev and fast_next later
type PlayerIcons = 'prev' | 'playpause' | 'next';

interface PlayerBarProps {
  iconsVisibility?: Record<PlayerIcons, boolean>;
  onClickRestArea?: () => void;
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
        <PlayerSlider valueMs={currentPositionMs()} maxMs={durationMs()} disabled={!canSeek()} onPreview={previewSeek} onCommit={seek} />
      </div>

      <Show when={coverArt()}>
        {(dataUrl) => <img class='absolute w-full h-full object-cover object-center' src={dataUrl()} loading='lazy' decoding='async' />}
      </Show>
      <div class='absolute z-0 w-full h-full  backdrop-blur-xs'></div>
      <div class='absolute z-0 w-full h-full bg-primary-plane opacity-75'></div>
      <div class='absolute z-0 w-full h-full  from-primary-plane to-transparent bg-linear-to-r'></div>

      <div class='relative flex flex-row w-full h-full px-4 gap-2 items-center shadow-xl z-10'>
        <div
          class='absolute z-0 w-full h-full'
          onClick={() => {
            props.onClickRestArea?.();
          }}
        ></div>
        <div class='flex flex-row gap-2 items-center'>
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

        <Switch
          fallback={
            <>
              <div class='flex min-w-0 flex-col flex-1 ml-2'>
                <MarqueeParagraph text={currentEntry()?.title || '[unknown]'} class='archivo font-bold' />
                <MarqueeParagraph text={subtitleText()} class='archivo italic text-xs leading-none text-secondary-text' pixelsPerSecond={40} />
              </div>
              <div class='flex flex-col gap-1 mr-1'>
                <p class='archivo text-xs'>
                  {currentPositionText()} / {durationText()}
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
