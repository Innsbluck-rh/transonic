import { Component, createMemo, JSX, Match, Show, Switch } from 'solid-js';
import { type SongResponse } from '~/bindings';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { usePlayback } from '~/features/playback/usePlayback';
import PlayerIcon from './PlayerIcon';
import PlayerSlider from './PlayerSlider';

// this may include fast_prev and fast_next later
type PlayerIcons = 'prev' | 'playpause' | 'next';

interface PlayerBarProps {
  iconsVisibility?: Record<PlayerIcons, boolean>;
  onClickTitle?: (entry?: SongResponse) => void;
  onClickArtist?: (entry?: SongResponse) => void;
  onClickRestArea?: () => void;
  restAreaProps?: JSX.HTMLAttributes<HTMLDivElement>;
}

const PlayerBar: Component<PlayerBarProps> = (props) => {
  const navigate = useSPNavigate();

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

  const { src: coverArt } = useCoverArt(() => currentEntry()?.coverArtId, CoverArtSizes.md, {
    cachedFallbackSizes: [CoverArtSizes.lg],
  });
  const restAreaClass = createMemo(() => props.restAreaProps?.class);

  const onClickRestArea: JSX.EventHandlerUnion<HTMLDivElement, MouseEvent> = (event) => {
    event.stopPropagation();
    props.onClickRestArea?.();
  };

  return (
    <div class='relative flex h-24 w-full flex-col shadow-[0_-1px_2px_0_rgb(0_0_0/0.05)]'>
      <div class='absolute z-30 w-full translate-y-[-50%]'>
        <PlayerSlider valueMs={currentPositionMs()} maxMs={durationMs()} disabled={!canSeek()} onPreview={previewSeek} onCommit={seek} />
      </div>

      <Show when={coverArt()}>
        {(assetUrl) => <img class='absolute h-full w-full object-cover object-center' src={assetUrl()} loading='lazy' decoding='async' />}
      </Show>
      <div class='absolute z-0 h-full w-full backdrop-blur-[2px]'></div>
      <div class='bg-primary-plane absolute z-0 h-full w-full opacity-50'></div>
      <div class='from-primary-plane absolute z-0 h-full w-full bg-linear-to-r to-transparent'></div>

      <div class='relative z-10 flex h-full w-full flex-row items-center gap-2.5 px-3.5 shadow-xl lg:px-6'>
        <div class='z-10 flex flex-row items-center gap-2'>
          <Show when={iconsVisibility.prev}>
            <PlayerIcon iconClass='p-1' type='prev' disabled={isControlDisabled()} onClick={prev} />
          </Show>
          <Show when={iconsVisibility.playpause}>
            <PlayerIcon
              iconClass='p-1'
              type={playingState() === 'playing' ? 'pause' : 'play'}
              disabled={isControlDisabled()}
              loading={isInterrupted()}
              onClick={togglePlayPause}
            />
          </Show>
          <Show when={iconsVisibility.next}>
            <PlayerIcon iconClass='p-1' type='next' disabled={isControlDisabled()} onClick={next} />
          </Show>
        </div>

        <div {...props.restAreaProps} class={`ml-3 flex min-w-0 flex-1 items-center ${restAreaClass() ?? ''}`} onClick={onClickRestArea}>
          <Switch
            fallback={
              <>
                <div class='flex min-w-0 flex-1 flex-col'>
                  <MarqueeParagraph
                    text={currentEntry()?.title || '[unknown]'}
                    class='archivo w-fit text-lg font-bold'
                    classList={{
                      'cursor-pointer': !!props.onClickTitle,
                    }}
                    onClick={() => {
                      props.onClickTitle?.(currentEntry() ?? undefined);
                    }}
                  />
                  <MarqueeParagraph
                    text={subtitleText()}
                    class='archivo text-secondary-text w-fit text-xs'
                    classList={{
                      'cursor-pointer': !!props.onClickArtist,
                    }}
                    pixelsPerSecond={40}
                    onClick={() => {
                      props.onClickArtist?.(currentEntry() ?? undefined);
                    }}
                  />
                </div>
                <div class='ml-4 flex flex-col gap-1'>
                  <p class='archivo text-sm'>
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
