import { Component, Show } from 'solid-js';
import { resolveSPBrowseRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { connectStore } from '~/stores/ConnectStore';
import ConnectButton from '../common/ConnectButton';
import Title from '../common/Title';

const SPHeader: Component = () => {
  const navigate = useSPNavigate();

  return (
    <div class='bg-primary-plane z-50 flex h-12 flex-row items-center pr-2.5 pl-1.5 shadow-md' style={{ 'view-transition-name': 'sp-header' }}>
      <Title class='cursor-pointer' onClick={() => navigate(resolveSPBrowseRoute())}>
        Transonic
      </Title>

      <div class='flex-1' />

      <Show when={connectStore.status === 'available'}>
        <ConnectButton />
      </Show>
      {/*
      <div class='flex flex-row gap-0'>
        <div class='relative overflow-visible'>
          <div
            class='bg-primary-plane hover:bg-primary-hover flex cursor-pointer flex-row items-center gap-2 rounded-full p-2'
            onClick={() => navigate(resolveSettingsRoute())}
          >
            <Icon class='text-primary-text text-[16px]' icon='pixelarticons:settings-2-sharp' />
          </div>
        </div>
      </div>*/}
    </div>
  );
};

export default SPHeader;
