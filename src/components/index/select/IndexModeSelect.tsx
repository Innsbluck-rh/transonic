import { Component, createMemo, createSignal, For, Show } from 'solid-js';

import { BROWSE_MODE_URLS, BrowseMode } from '../IndexSideBar';
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
      <div
        class='flex flex-row w-full cursor-pointer border-b border-zinc-400 bg-zinc-50 px-3 py-2 text-left hover:bg-zinc-100'
        onClick={() => setOpen((current) => !current)}
      >
        <p class='archivo font-bold text-zinc-800 text-xs'>{browseMode()}</p>

        <div
          class={`opacity-50 w-0 h-0 self-center ml-auto
            border-l-[5px] border-l-transparent 
            border-r-[5px] border-r-transparent 
            border-t-[5px] border-t-zinc-800 ${selectStates.toggle}`}
          classList={{ [selectStates['is-open']]: open() }}
        ></div>
      </div>
      <div class={`grid ${selectStates.panel}`} classList={{ [selectStates['is-open']]: open() }}>
        <div class={`relative flex flex-col ${selectStates['panel-inner']}`}>
          {/* top shadow */}
          <div class='pointer-events-none absolute top-0 h-3 w-full bg-linear-to-t from-transparent to-zinc-500 opacity-10' />
          <For each={otherModes()}>
            {(mode) => (
              <div
                class='flex w-full cursor-pointer bg-zinc-50 px-3 py-2 hover:bg-zinc-200'
                onClick={() => {
                  props.onSelect?.(mode);
                  setOpen(false);
                }}
              >
                <p class='archivo font-bold text-zinc-800 text-xs'>{mode}</p>
              </div>
            )}
          </For>
          {/* bottom shadow (w/safety margin) */}
          <div class='pointer-events-none absolute -bottom-4 h-8 w-full bg-linear-to-b from-transparent to-zinc-500 opacity-20' />
        </div>
      </div>

      {/* border */}
      <Show when={open()}>
        <div class='w-full h-0 border-b border-zinc-400' />
      </Show>
    </div>
  );
};

export default IndexModeSelect;
