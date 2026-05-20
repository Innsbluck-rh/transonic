import { Component, createMemo, JSX, Match, Show, Switch } from 'solid-js';
import { type SongResponse } from '~/bindings';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { usePlayback } from '~/features/playback/usePlayback';
import PlayerIcon from '../player/PlayerIcon';

// this may include fast_prev and fast_next later
type PlayerIcons = 'prev' | 'playpause' | 'next';

interface PlayerBarProps {
  iconsVisibility?: Record<PlayerIcons, boolean>;
  onClickTitle?: (entry?: SongResponse) => void;
  onClickArtist?: (entry?: SongResponse) => void;
  onClickRestArea?: () => void;
  restAreaProps?: JSX.HTMLAttributes<HTMLDivElement>;
}

const SPPlayerBar: Component<PlayerBarProps> = (props) => {
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
    interruptLabel: playbackInterruptLabel,
    togglePlayPause,
    progressPercent,
    prev,
    next,
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
    <div class='bg-primary-plane ripple relative flex w-full flex-col overflow-hidden rounded-xl shadow-[0_1px_8px_0_rgb(0_0_0/0.25)]'>
      <Show when={coverArt()}>
        {(assetUrl) => <img class='absolute h-full w-full object-cover object-center' src={assetUrl()} loading='lazy' decoding='async' />}
      </Show>
      <div class='absolute z-0 h-full w-full backdrop-blur-[2px]'></div>
      <div class='bg-primary-plane absolute z-0 h-full w-full opacity-50'></div>
      <div
        class='bg-primary-playing absolute z-0 h-full opacity-80'
        style={{
          width: `${progressPercent()}%`,
        }}
      ></div>
      <div class='relative z-10 flex h-full w-full flex-row items-center gap-3 px-4 py-2.5 shadow-xl lg:p-5 lg:px-6'>
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

        <div {...props.restAreaProps} class={`flex min-w-0 flex-1 items-center ${restAreaClass() ?? ''}`} onClick={onClickRestArea}>
          <Switch
            fallback={
              <>
                <div class='flex min-w-0 flex-1 flex-col'>
                  <MarqueeParagraph
                    text={currentEntry()?.title || '[unknown]'}
                    class='archivo text-primary-text -mb-0.5 w-fit text-lg font-black tracking-tighter'
                    classList={{
                      'cursor-pointer': !!props.onClickTitle,
                    }}
                  />
                  <MarqueeParagraph
                    text={subtitleText()}
                    class='archivo text-secondary-text w-fit text-[13px]'
                    classList={{
                      'cursor-pointer': !!props.onClickArtist,
                    }}
                  />
                </div>
              </>
            }
          >
            <Match when={playingState() === 'error'}>
              <div class='flex-1'>
                <p class='archivo text-secondary-text italic'>[error occured]</p>
              </div>
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  );
};

export default SPPlayerBar;
