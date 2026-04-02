import { Component, createMemo, createSignal, For, Show } from 'solid-js';

import { BROWSE_MODE_URLS, BrowseMode } from '../IndexContent';
import selectStates from './select_states.module.css';

interface IndexModeSelectProps {
  defaultMode: BrowseMode;
  onSelect?: (mode: BrowseMode) => void;
}

const IndexModeSelect: Component<IndexModeSelectProps> = (props) => {
  const browseMode = createMemo(() => props.defaultMode);
  const [open, setOpen] = createSignal(false);
  const otherModes = createMemo<BrowseMode[]>(() => Object.keys(BROWSE_MODE_URLS).filter((mode): mode is BrowseMode => mode !== browseMode()));

  return (
    <div class='flex flex-col'>
      <div class='flex flex-row w-full cursor-pointer px-3 py-2 text-left hover:bg-primary-hover' onClick={() => setOpen((current) => !current)}>
        <p class='archivo font-bold text-xs'>{browseMode()}</p>

        <div
          class={`opacity-50 w-0 h-0 self-center ml-auto
            border-l-[5px] border-l-transparent 
            border-r-[5px] border-r-transparent 
            border-t-[5px] border-t border-secondary-text ${selectStates.toggle}`}
          classList={{ [selectStates['is-open']]: open() }}
        ></div>
      </div>
      <div class={`grid bg-primary-surface ${selectStates.panel}`} classList={{ [selectStates['is-open']]: open() }}>
        <div class={`relative flex flex-col ${selectStates['panel-inner']}`}>
          <Show when={open()}>
            <div class='h-0 w-full border-t border-secondary-border' />
          </Show>
          {/* top shadow */}
          {/* <div class='pointer-events-none absolute top-0 h-3 w-full bg-linear-to-t from-transparent to-zinc-500 opacity-10' /> */}
          <For each={otherModes()}>
            {(mode) => (
              <div
                class='flex w-full cursor-pointer px-3 py-2 hover:bg-primary-hover'
                onClick={() => {
                  props.onSelect?.(mode);
                  setOpen(false);
                }}
              >
                <p class='archivo font-bold text-xs'>{mode}</p>
              </div>
            )}
          </For>
          {/* bottom shadow (w/safety margin) */}
          {/* <div class='pointer-events-none absolute -bottom-4 h-8 w-full bg-linear-to-b from-transparent to-zinc-500 opacity-20' /> */}
        </div>
      </div>

      {/* border */}
      <div class='w-full h-0 border-b border-primary-border' />
    </div>
  );
};

export default IndexModeSelect;
