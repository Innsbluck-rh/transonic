import { Icon } from '@iconify-icon/solid';
import { Component, createSignal, Show } from 'solid-js';
import { commands, type ConnectDevicePresence } from '~/bindings';
import { connectStore } from '~/stores/ConnectStore';

type ConnectDeviceCardProps = {
  device: ConnectDevicePresence;
};

const ConnectDeviceCard: Component<ConnectDeviceCardProps> = (props) => {
  const [busy, setBusy] = createSignal(false);
  const [actionError, setActionError] = createSignal<string | null>(null);
  const isActive = () => connectStore.sharedPlayback?.activeDeviceId === props.device.deviceId;
  const isPlaying = () => isActive() && connectStore.sharedPlayback?.state.playingState === 'playing';

  const canTransfer = () =>
    connectStore.runtime.connected &&
    connectStore.capabilities?.playbackTransfer === true &&
    props.device.online &&
    connectStore.sharedPlayback !== null &&
    !busy() &&
    !isActive();

  const transferPlayback = async () => {
    setBusy(true);
    setActionError(null);
    const result = await commands.connectTransferPlayback({ targetDeviceId: props.device.deviceId });
    if (result.status === 'error') {
      setActionError(result.error);
    }
    setBusy(false);
  };

  return (
    <div
      class='hover:bg-primary-hover flex min-h-8 min-w-0 items-center gap-2 overflow-hidden px-3 py-2'
      classList={{
        'bg-transparent hover:bg-transparent': isActive(),
        'bg-transparent': !isActive(),
        'cursor-pointer': canTransfer(),
        'cursor-default': !canTransfer(),
      }}
      onClick={() => {
        if (canTransfer()) void transferPlayback();
      }}
    >
      <div class='flex h-full w-5 items-center'>
        <Icon
          icon={isPlaying() ? 'material-symbols:volume-up' : 'material-symbols:devices'}
          class={'text-[18px]'}
          classList={{
            'text-accent': isActive(),
            'text-primary-text': !isActive() && props.device.online,
            'text-secondary-text': !isActive() && !props.device.online,
          }}
        />
      </div>

      <div class='flex h-full min-w-0 flex-1 items-center overflow-hidden'>
        <p
          class='min-w-0 text-sm leading-none font-bold'
          classList={{
            'text-accent': isActive(),
            'text-primary-text': !isActive() && props.device.online,
            'text-secondary-text': !isActive() && !props.device.online,
          }}
        >
          {props.device.displayName || 'Unnamed device'}
        </p>
      </div>

      <Show when={actionError()}>{(message) => <p class='text-secondary-text min-w-0 text-xs break-words'>{message()}</p>}</Show>
    </div>
  );
};

export default ConnectDeviceCard;
