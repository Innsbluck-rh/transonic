import { Component, createSignal, For } from 'solid-js';

import { BROWSE_MODES, type BrowseMode } from '~/features/navigation/routes';

interface IndexModeSelectProps {
  defaultMode: BrowseMode;
  onSelect?: (mode: BrowseMode) => void;
}

const IndexModeSelect2: Component<IndexModeSelectProps> = (props) => {
  const [currentMode, setCurrentMode] = createSignal<BrowseMode>(props.defaultMode);

  return (
    <div class='flex flex-col'>
      <select
        class='p-1.5'
        value={currentMode()}
        onChange={(e) => {
          setCurrentMode(e.currentTarget.value as BrowseMode);
          props.onSelect?.(e.currentTarget.value as BrowseMode);
        }}
      >
        <For each={BROWSE_MODES}>{(key) => <option value={key}>{key}</option>}</For>
      </select>
    </div>
  );
};

export default IndexModeSelect2;
