import { Component, createMemo, createSignal, For } from 'solid-js';

import { Icon } from '@iconify-icon/solid';
import { BROWSE_MODES, type BrowseMode } from '~/features/navigation/routes';
import selectStates from './select_states.module.css';

interface IndexModeSelectProps {
  defaultMode: BrowseMode;
  onSelect?: (mode: BrowseMode) => void;
}

const IndexModeSelect: Component<IndexModeSelectProps> = (props) => {
  const browseMode = createMemo(() => props.defaultMode);
  const [open, setOpen] = createSignal(false);
  const otherModes = createMemo<BrowseMode[]>(() => BROWSE_MODES.filter((mode) => mode !== browseMode()));

  return (
    <div class='bg-primary-surface border-secondary-border flex w-full flex-col rounded-lg border shadow-lg'>
      <div
        class='hover:bg-primary-hover flex w-full cursor-pointer flex-row items-center gap-2 px-3 py-1.5'
        onClick={() => setOpen((current) => !current)}
      >
        <Icon class='text-sm' icon={getIconForMode(browseMode())} />
        <p class='archivo flex-1 text-xs font-bold'>{browseMode()}</p>
        <Icon class='ml-auto w-4 text-center text-xl' icon={open() ? 'material-symbols:arrow-drop-up' : 'material-symbols:arrow-drop-down'} />
      </div>

      <div class={`bg-primary-surface grid ${selectStates.panel}`} classList={{ [selectStates['is-open']]: open() }}>
        <div class={`relative flex flex-col ${selectStates['panel-inner']}`}>
          <For each={otherModes()}>
            {(mode) => (
              <div
                class='hover:bg-primary-hover flex w-full cursor-pointer items-center gap-2 px-3 py-2'
                onClick={() => {
                  props.onSelect?.(mode);
                  setOpen(false);
                }}
              >
                <Icon class='text-sm' icon={getIconForMode(mode)} />
                <p class='archivo text-xs'>{mode}</p>
              </div>
            )}
          </For>
        </div>
      </div>

      {/* border */}
      <div class='border-primary-border h-0 w-full' />
    </div>
  );
};

const getIconForMode = (mode: BrowseMode) => {
  switch (mode) {
    case 'Folder Structures':
      // return "material-symbols:folder";
      return 'material-symbols:folder-copy-rounded';
    case 'Artists':
      return 'material-symbols:artist-rounded';
  }
};

export default IndexModeSelect;
