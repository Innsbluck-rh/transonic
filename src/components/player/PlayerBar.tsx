import { Icon } from '@iconify-icon/solid';
import { platform } from '@tauri-apps/plugin-os';
import { Component, createMemo, JSX, Show } from 'solid-js';
import { ConnectDevicePresence, type SongResponse } from '~/bindings';
import MarqueeParagraph from '~/components/common/text/MarqueeParagraph';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { resolveExternalPlaybackDevice } from '~/features/connect/playbackDevice';
import { usePlayback } from '~/features/playback/usePlayback';
import { usePlaybackStatus } from '~/features/playback/usePlaybackStatus';
import { connectStore } from '~/stores/ConnectStore';
import { openPlaybackStreamInfoDialog } from '../common/dialog/PlaybackStreamInfoDialog';
import LoadCircle from '../common/LoadCircle';
import PlaybackStatusText from '../player/common/PlaybackStatusText';
import VolumeSlider from '../player/common/VolumeSlider';
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
    volume,
    previewVolume,
    commitPreviewedVolume,
  } = usePlayback();
  const playbackStatus = usePlaybackStatus();

  const externalPlaybackDevice = createMemo<ConnectDevicePresence | undefined>(() => {
    return resolveExternalPlaybackDevice({
      connected: connectStore.runtime.connected,
      ownDeviceId: connectStore.runtime.deviceId,
      devices: connectStore.devices,
      sharedPlayback: connectStore.sharedPlayback,
    });
  });

  const isWindows = platform() === 'windows';
  const subtitleText = createMemo(() => {
    const artist = currentEntry()?.artist || '[unknown]';
    return isInterrupted() ? `${artist} ${playbackInterruptLabel()}` : artist;
  });

  const { src: coverArt } = useCoverArt(() => currentEntry()?.coverArtId, CoverArtSizes.lg, {
    cachedFallbackSizes: [CoverArtSizes.lg],
  });
  const restAreaClass = createMemo(() => props.restAreaProps?.class);

  const onClickRestArea: JSX.EventHandlerUnion<HTMLDivElement, MouseEvent> = (event) => {
    event.stopPropagation();
    props.onClickRestArea?.();
  };

  return (
    <div class='bg-primary-plane relative flex h-[92px] w-full flex-col shadow-[0_-1px_2px_0_rgb(0_0_0/0.05)]'>
      <div class='absolute z-30 w-full translate-y-[-50%]'>
        <PlayerSlider valueMs={currentPositionMs()} maxMs={durationMs()} disabled={!canSeek()} onPreview={previewSeek} onCommit={seek} />
      </div>

      <Show when={coverArt()}>
        {(assetUrl) => <img class='absolute size-full object-cover object-center' src={assetUrl()} loading='lazy' decoding='async' />}
      </Show>
      <div class='absolute z-0 size-full backdrop-blur-[2px]'></div>
      <div class='bg-primary-plane absolute z-0 size-full opacity-50'></div>
      <div class='from-primary-plane absolute z-0 size-full bg-linear-to-r to-transparent'></div>

      <div class='relative z-10 flex size-full flex-row items-center gap-2.5 px-3.5 shadow-xl lg:px-4'>
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
          <Show
            when={playingState() !== 'idle'}
            fallback={
              <div class='flex-1'>
                <p class='archivo text-secondary-text italic'>[nothing played]</p>
              </div>
            }
          >
            <div class='relative mr-4.5 aspect-square h-[48px] w-auto'>
              <Show
                when={coverArt()}
                fallback={
                  <div class='border-secondary-border absolute inset-0 flex size-full items-center justify-center overflow-hidden rounded-sm border object-center'>
                    <LoadCircle />
                  </div>
                }
              >
                {(assetUrl) => (
                  <img
                    class='absolute inset-0 size-full overflow-hidden rounded-sm object-cover object-center'
                    src={assetUrl()}
                    loading='lazy'
                    decoding='async'
                  />
                )}
              </Show>
            </div>
            <div class='flex min-w-0 flex-1 flex-col gap-1'>
              <MarqueeParagraph
                text={currentEntry()?.title || '[unknown]'}
                class='archivo mt-1 w-fit pb-1 text-lg leading-none font-bold'
                classList={{
                  'cursor-pointer': !!props.onClickTitle,
                }}
                onClick={() => {
                  props.onClickTitle?.(currentEntry() ?? undefined);
                }}
              />
              <div class='flex h-full w-full items-center gap-1'>
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

                <Show when={externalPlaybackDevice()}>
                  <>
                    <p class='text-secondary-text leading-none'>・</p>
                    <Icon icon={'material-symbols:volume-up'} class='text-accent text-sm leading-none' />
                    <p class='text-accent text-xs leading-none font-bold'>{externalPlaybackDevice()?.displayName}</p>
                  </>
                </Show>
              </div>
            </div>
            <Show when={isWindows}>
              <VolumeSlider volume={volume()} onVolumeInput={previewVolume} onVolumeCommit={commitPreviewedVolume} />
            </Show>
            <div class='mr-2 ml-5 flex min-w-22 flex-col gap-1 text-end'>
              <p class='text-sm'>
                {currentPositionText()} / <span class='font-bold'>{durationText()}</span>
              </p>
              <div class='flex items-center gap-1.5'>
                <PlaybackStatusText class='ml-auto' />
                <Show when={playbackStatus()?.actualStreamInfo}>
                  <div
                    class='ripple flex cursor-pointer flex-row items-center rounded-full p-0.5'
                    title='Playback stream info'
                    aria-label='Playback stream info'
                    onClick={() => openPlaybackStreamInfoDialog()}
                  >
                    <Icon class='text-primary-text' icon='material-symbols:info-outline' />
                  </div>
                </Show>
              </div>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
};

export default PlayerBar;
