import { Component, createMemo, JSX, Match, Show, Switch } from 'solid-js';
import { type SongResponse } from '~/bindings';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
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

  const { src: coverArt } = useCoverArt(() => currentEntry()?.coverArtId);
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
      <div class='absolute z-0 h-full w-full backdrop-blur-[2px]'></div>
      <div class='bg-primary-plane absolute z-0 h-full w-full opacity-50'></div>
      <div class='from-primary-plane absolute z-0 h-full w-full bg-linear-to-r to-transparent'></div>

      <div class='relative z-10 flex h-full w-full flex-row items-center gap-3 p-3.5 shadow-xl lg:p-5 lg:px-6'>
        <div class='z-10 flex flex-row items-center gap-3'>
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
                  <MarqueeParagraph
                    text={currentEntry()?.title || '[unknown]'}
                    class='archivo w-fit leading-none font-bold'
                    classList={{
                      'cursor-pointer': !!props.onClickTitle,
                    }}
                    onClick={() => {
                      props.onClickTitle?.(currentEntry() ?? undefined);
                    }}
                  />
                  <MarqueeParagraph
                    text={subtitleText()}
                    class='archivo text-secondary-text w-fit text-xs leading-none'
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
