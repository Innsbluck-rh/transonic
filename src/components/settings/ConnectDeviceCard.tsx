import { createMemo, createSignal, Show } from 'solid-js';
import { commands } from '~/bindings';
import { connectStore } from '~/stores/ConnectStore';
import { formatCompactDuration } from '~/utils/duration';

type ConnectDeviceEntry = (typeof connectStore.devices)[number];

type ConnectDeviceCardProps = {
  entry: ConnectDeviceEntry;
  coverArtSrc: (coverArtId: string | null | undefined) => string | null | undefined;
};

function formatPosition(ms: number) {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

export function currentSong(playback: NonNullable<ConnectDeviceEntry['playback']>) {
  const queue = playback.state.queue ?? [];
  const index = playback.state.currentIndex;
  if (index != null) {
    const song = queue[index];
    if (song) {
      return song;
    }
  }
  return queue.find((song) => song.id === playback.state.currentSongId) ?? null;
}

function ConnectDeviceCard(props: ConnectDeviceCardProps) {
  const [busyAction, setBusyAction] = createSignal<'take' | 'pause' | null>(null);
  const [actionError, setActionError] = createSignal<string | null>(null);
  const song = createMemo(() => (props.entry.playback ? currentSong(props.entry.playback) : null));
  const coverArt = () => props.coverArtSrc(song()?.coverArtId);
  const isSelf = () => props.entry.device.deviceId === connectStore.runtime.deviceId;
  const canControl = () => connectStore.runtime.connected && props.entry.device.online && !isSelf();
  const canTake = () => canControl() && !!props.entry.playback && (props.entry.playback.state.queue ?? []).length > 0;
  const canPause = () => canControl() && !!props.entry.playback && props.entry.playback.state.playingState !== 'idle';

  const runAction = async (action: 'take' | 'pause') => {
    setBusyAction(action);
    setActionError(null);
    const result =
      action === 'take'
        ? await commands.connectTakeoverPlayback({ sourceDeviceId: props.entry.device.deviceId })
        : await commands.connectPauseDevicePlayback({ targetDeviceId: props.entry.device.deviceId });
    if (result.status === 'error') {
      setActionError(result.error);
    }
    setBusyAction(null);
  };

  return (
    <div class='border-primary-border bg-primary-surface flex min-w-0 flex-col gap-2 overflow-hidden rounded border px-3 py-2.5'>
      <div class='flex min-w-0 items-center gap-2'>
        <p class='min-w-0 flex-1 truncate text-sm font-bold'>{props.entry.device.displayName || 'Unnamed device'}</p>
        <p class={props.entry.device.online ? 'text-accent shrink-0 text-xs' : 'text-secondary-text shrink-0 text-xs'}>
          {props.entry.device.online ? 'online' : 'offline'}
        </p>
      </div>

      <div class='flex min-w-0 gap-3 overflow-hidden' classList={{ 'opacity-50': !props.entry.device.online }}>
        <Show when={coverArt()} fallback={<div class='bg-primary-plane border-primary-border aspect-square h-12 w-12 shrink-0 rounded border' />}>
          {(assetUrl) => <img src={assetUrl()} class='aspect-square h-12 w-12 shrink-0 rounded object-cover' loading='lazy' decoding='async' />}
        </Show>

        <div class='min-w-0 flex-1 overflow-hidden'>
          <Show when={props.entry.playback && song()} fallback={<p class='text-secondary-text truncate text-xs'>No playback</p>}>
            {(current) => (
              <div class='mt-1 flex min-w-0 flex-col gap-1 overflow-hidden'>
                <p class='min-w-0 truncate text-sm' title={current().title}>
                  {current().title}
                </p>
                <Show when={current().artist}>
                  {(artist) => (
                    <p class='text-secondary-text min-w-0 truncate text-xs' title={artist()}>
                      {artist()}
                    </p>
                  )}
                </Show>
              </div>
            )}
          </Show>
        </div>

        <Show when={props.entry.playback}>
          {(playback) => (
            <div class='text-secondary-text flex max-w-[6.5rem] shrink-0 flex-col items-end gap-1 overflow-hidden text-xs'>
              <p
                class='max-w-full truncate text-sm'
                title={`${formatPosition(playback().positionMs)} / ${formatCompactDuration(song()?.duration ?? 0)}`}
              >
                {formatPosition(playback().positionMs)}
                <Show when={song()?.duration}> / {formatCompactDuration(song()?.duration ?? 0)}</Show>
              </p>
              <p class='max-w-full truncate text-xs'>{playback().state.playingState}</p>
            </div>
          )}
        </Show>
      </div>

      <Show when={!isSelf()}>
        <div class='flex min-w-0 flex-wrap items-center gap-2'>
          <button type='button' disabled={!canTake() || busyAction() !== null} class='px-2 py-1 text-xs' onClick={() => void runAction('take')}>
            {busyAction() === 'take' ? 'Taking' : 'Take'}
          </button>
          <button type='button' disabled={!canPause() || busyAction() !== null} class='px-2 py-1 text-xs' onClick={() => void runAction('pause')}>
            {busyAction() === 'pause' ? 'Pausing' : 'Pause'}
          </button>
          <Show when={actionError()}>{(message) => <p class='text-secondary-text min-w-0 text-xs break-words'>{message()}</p>}</Show>
        </div>
      </Show>
    </div>
  );
}

export default ConnectDeviceCard;
