import { Component, createEffect, createSignal, Show } from 'solid-js';
import { commands, type ConnectQueueResolutionChoice, type PlayingState, type SongResponse } from '~/bindings';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { setConnectSetting } from '~/features/settings/service';
import { connectStore } from '~/stores/ConnectStore';
import { playbackStore } from '~/stores/PlaybackStore';
import { formatCompactDuration } from '~/utils/duration';
import Heading3 from '../text/Heading3';

// Shown while the Rust side holds the local queue back because it conflicts with
// the Connect server's queue on connect. Cancel turns Connect off again.
export default function ConnectQueueResolveDialog() {
  const [requestError, setRequestError] = createSignal<string | null>(null);
  const resolution = () => connectStore.runtime.queueResolution;
  const pushing = () => resolution()?.pushing === true;
  const error = () => requestError() ?? resolution()?.error ?? null;

  createEffect(() => {
    if (!resolution()) {
      setRequestError(null);
    }
  });

  const resolve = async (choice: ConnectQueueResolutionChoice) => {
    setRequestError(null);
    const result = await commands.connectResolveQueue({ choice });
    if (result.status === 'error') {
      setRequestError(result.error);
    }
  };

  const cancel = () => {
    setRequestError(null);
    void setConnectSetting('enabled', false);
  };

  return (
    <Show when={resolution()}>
      <div class='fixed inset-0 z-100 flex items-center justify-center bg-black/45 p-4' role='presentation'>
        <section
          class='border-primary-border bg-primary-plane text-primary-text flex max-h-[80dvh] w-full max-w-2xl flex-col overflow-hidden rounded border shadow-xl'
          role='alertdialog'
          aria-modal='true'
          aria-labelledby='connect-queue-resolve-title'
        >
          <header class='border-secondary-border flex items-center gap-3 border-b px-4 py-3'>
            <h2 id='connect-queue-resolve-title' class='text-primary-text min-w-0 flex-1 text-base'>
              Resolve Queue
            </h2>
            <button class='button-borderless px-2' type='button' onClick={cancel} aria-label='Cancel queue resolution'>
              <i class='icon-[material-symbols--close]' />
            </button>
          </header>
          <div class='flex flex-col gap-1 p-4'>
            <ResolvingQueueItem
              name='Server'
              icon='icon-[material-symbols--host-sharp]'
              queue={connectStore.sharedPlayback?.state.queue ?? []}
              currentIndex={connectStore.sharedPlayback?.state.currentIndex}
              playingState={connectStore.sharedPlayback?.state.playingState}
            />

            <div class='flex items-center justify-end justify-evenly gap-2 px-4 py-3'>
              <button class='flex items-center gap-1' type='button' disabled={pushing()} onClick={() => void resolve('pushLocal')}>
                <i class='icon-[material-symbols--arrow-upward]' />
                <span>Push Local</span>
              </button>
              <button class='flex items-center gap-1' type='button' disabled={pushing()} onClick={() => void resolve('pullServer')}>
                <i class='icon-[material-symbols--arrow-downward]' />
                <span>Pull From Server</span>
              </button>
            </div>

            <ResolvingQueueItem
              name='Local'
              icon='icon-[material-symbols--devices-sharp]'
              queue={playbackStore.status?.queue ?? []}
              currentIndex={playbackStore.status?.currentIndex}
              playingState={playbackStore.status?.playingState}
            />
          </div>

          <footer class='border-secondary-border flex items-center justify-end gap-2 border-t px-4 py-3'>
            <Show when={error()}>{(message) => <span class='mr-auto text-xs text-red-500'>{message()}</span>}</Show>
            <button type='button' onClick={cancel}>
              Cancel
            </button>
          </footer>
        </section>
      </div>
    </Show>
  );
}

interface ResolvingQueueItemProps {
  name: string;
  icon: string;
  queue: SongResponse[];
  currentIndex?: number | null;
  playingState?: PlayingState;
}

const ResolvingQueueItem: Component<ResolvingQueueItemProps> = (props) => {
  const entry = () => props.queue[props.currentIndex ?? 0];
  const coverArt = useCoverArt(() => entry()?.coverArtId, CoverArtSizes.sm, {
    cachedFallbackSizes: [CoverArtSizes.lg, CoverArtSizes.md],
  });

  return (
    <div class='flex cursor-pointer flex-col gap-2 rounded-md px-3 py-2'>
      <div class='flex items-center gap-2'>
        <i class={props.icon} />
        <Heading3>{props.name}</Heading3>
        <div class='flex-1' />
        <p class='font-bold'>
          {props.queue.length} items / {props.playingState ?? 'idle'}
        </p>
      </div>
      <Show when={entry()}>
        {(song) => (
          <div class='bg-primary-surface ripple relative flex h-auto flex-row items-center overflow-x-hidden rounded-md px-3 py-2'>
            <Show when={coverArt.src()} fallback={<div class='border-secondary-border mr-3 aspect-square max-h-8 w-8 rounded-md border' />}>
              {(url) => <img src={url()} class='mr-3 aspect-square max-h-8 w-8 rounded-md' loading='lazy' decoding='async' />}
            </Show>

            <div class='flex min-w-0 flex-1 flex-col gap-0'>
              <div class='flex items-center gap-1'>
                <p class='song-item-title archivo gap-1 truncate font-bold'>{song().title}</p>
              </div>
              <p class='song-item-meta archivo truncate text-[11px]'>{song().artist}</p>
            </div>

            <p class='song-item-meta archivo ml-2 text-xs'>{formatCompactDuration(song().duration ?? 0)}</p>
          </div>
        )}
      </Show>
    </div>
  );
};
