import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, JSX, Show } from 'solid-js';
import { ConnectDevicePresence, type SongResponse } from '~/bindings';
import MarqueeParagraph from '~/components/common/text/MarqueeParagraph';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { resolveExternalPlaybackDevice } from '~/features/connect/playbackDevice';
import { usePlayback } from '~/features/playback/usePlayback';
import { connectStore } from '~/stores/ConnectStore';
import PlayerIcon from '../../player/PlayerIcon';

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

  const externalPlaybackDevice = createMemo<ConnectDevicePresence | undefined>(() => {
    return resolveExternalPlaybackDevice({
      connected: connectStore.runtime.connected,
      ownDeviceId: connectStore.runtime.deviceId,
      devices: connectStore.devices,
      sharedPlayback: connectStore.sharedPlayback,
    });
  });

  return (
    <div class='bg-primary-plane flex w-full flex-col overflow-hidden rounded-xl shadow-[0px_0px_8px_rgba(0,0,0,0.25)]'>
      <div class='relative flex h-full w-full flex-col overflow-hidden'>
        <Show when={coverArt()}>
          {(assetUrl) => <img class='absolute size-full object-cover object-center' src={assetUrl()} loading='lazy' decoding='async' />}
        </Show>
        <div class='absolute z-0 size-full backdrop-blur-[2px]'></div>
        <div class='from-primary-plane/75 to-primary-plane/50 absolute z-0 size-full bg-linear-to-r'></div>
        {/*<div class='bg-primary-plane absolute z-0 size-full opacity-50'></div>*/}
        <div
          class='bg-accent absolute z-0 h-full opacity-50'
          style={{
            width: `${progressPercent()}%`,
          }}
        ></div>

        <div class='relative z-20 flex h-17 flex-row items-center'>
          <PlayerIcon
            type={playingState() === 'playing' ? 'pause' : 'play'}
            disabled={isControlDisabled()}
            loading={isInterrupted()}
            onClick={(e) => {
              e.stopPropagation();
              togglePlayPause();
            }}
            iconClass='p-5 m-0'
          />

          <div
            {...props.restAreaProps}
            class={`flex min-w-0 flex-1 flex-col items-center pr-3 pl-1 ${restAreaClass() ?? ''}`}
            onClick={onClickRestArea}
          >
            <MarqueeParagraph
              text={currentEntry()?.title || '[unknown]'}
              class='archivo text-primary-text mb-1 w-full text-lg leading-tight font-black tracking-tighter'
              classList={{
                'cursor-pointer': !!props.onClickTitle,
              }}
            />
            <div class='flex w-full items-center gap-1'>
              <MarqueeParagraph
                text={subtitleText()}
                class='archivo text-secondary-text w-fit text-[13px] leading-none'
                classList={{
                  'cursor-pointer': !!props.onClickArtist,
                }}
              />

              <Show when={externalPlaybackDevice()}>
                <>
                  <p class='text-secondary-text text-[13px] leading-none'>・</p>
                  <Icon icon={'material-symbols:volume-up'} class='text-accent text-[13px]' />
                  <p class='text-accent text-[13px] leading-none font-bold text-nowrap'>{externalPlaybackDevice()?.displayName}</p>
                </>
              </Show>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SPPlayerBar;
