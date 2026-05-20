import { Component, createMemo, createSignal, For, Show } from 'solid-js';

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
    <div class='flex flex-col'>
      <div class='hover:bg-primary-hover flex w-full cursor-pointer flex-row px-3 py-2 text-left' onClick={() => setOpen((current) => !current)}>
        <p class='archivo text-xs font-bold'>{browseMode()}</p>

        <div
          class={`border-secondary-text ml-auto h-0 w-0 self-center border-t border-t-[5px] border-r-[5px] border-l-[5px] border-r-transparent border-l-transparent opacity-50 ${selectStates.toggle}`}
          classList={{ [selectStates['is-open']]: open() }}
        ></div>
      </div>
      <div class={`bg-primary-surface grid ${selectStates.panel}`} classList={{ [selectStates['is-open']]: open() }}>
        <div class={`relative flex flex-col ${selectStates['panel-inner']}`}>
          <Show when={open()}>
            <div class='border-secondary-border h-0 w-full border-t' />
          </Show>
          {/* top shadow */}
          {/* <div class='pointer-events-none absolute top-0 h-3 w-full bg-linear-to-t from-transparent to-zinc-500 opacity-10' /> */}
          <For each={otherModes()}>
            {(mode) => (
              <div
                class='hover:bg-primary-hover flex w-full cursor-pointer px-3 py-2'
                onClick={() => {
                  props.onSelect?.(mode);
                  setOpen(false);
                }}
              >
                <p class='archivo text-xs font-bold'>{mode}</p>
              </div>
            )}
          </For>
          {/* bottom shadow (w/safety margin) */}
          {/* <div class='pointer-events-none absolute -bottom-4 h-8 w-full bg-linear-to-b from-transparent to-zinc-500 opacity-20' /> */}
        </div>
      </div>

      {/* border */}
      <div class='border-primary-border h-0 w-full border-b' />
    </div>
  );
};

export default IndexModeSelect;
