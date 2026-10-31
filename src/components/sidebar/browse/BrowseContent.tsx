import { Component, createSignal } from 'solid-js';
import { type BrowseMode } from '~/features/navigation/routes';
import BrowseItemList from './BrowseItemList';
import BrowseModeSelect from './BrowseModeSelect';

const BrowseContent: Component = () => {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Artists');

  return (
    <div class='relative flex size-full flex-col overflow-y-auto'>
      <div class='sticky top-0 z-1 box-border flex p-1'>
        <BrowseModeSelect
          defaultMode={browseMode()}
          onSelect={(mode) => {
            setBrowseMode(mode);
          }}
        />
      </div>
      <div class='flex flex-col'>
        <BrowseItemList mode={browseMode()} />
      </div>
    </div>
  );
};

export default BrowseContent;
