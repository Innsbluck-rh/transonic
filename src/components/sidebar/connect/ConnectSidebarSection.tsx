import { Component, For, Show } from 'solid-js';
import Heading3 from '~/components/common/text/Heading3';
import ConnectDeviceCard from '~/components/settings/ConnectDeviceCard';
import { connectStore } from '~/stores/ConnectStore';

const ConnectSidebarSection: Component = () => {
  return (
    <div class='bg-primary-plane flex min-h-42 w-full flex-col overflow-y-hidden'>
      <div class='border-secondary-border flex w-full flex-row items-center border-b px-2 py-1'>
        <Heading3>Connect</Heading3>
      </div>
      <Show when={connectStore.devices.length > 0}>
        <div class='flex flex-1 flex-col gap-1 overflow-y-auto p-2'>
          <For each={connectStore.devices}>{(device) => <ConnectDeviceCard device={device} />}</For>
        </div>
      </Show>
    </div>
  );
};

export default ConnectSidebarSection;
