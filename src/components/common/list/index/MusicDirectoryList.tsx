import { Component, For, Show } from 'solid-js';
import type { MusicDirectoryChild } from '~/bindings';

interface MusicDirectoryListProps {
  items: MusicDirectoryChild[];
  emptyMessage?: string;
  onClickDirectory?: (item: MusicDirectoryChild) => void;
}

function subtitle(item: MusicDirectoryChild) {
  const parts = [item.artist, item.year?.toString()].filter(Boolean);
  return parts.join(' / ');
}

const MusicDirectoryList: Component<MusicDirectoryListProps> = (props) => {
  return (
    <Show when={props.items.length > 0} fallback={<div class='px-2 py-6 text-sm text-zinc-400'>{props.emptyMessage ?? 'No items available.'}</div>}>
      <div class='flex flex-col divide-y divide-zinc-200 rounded-md border border-zinc-200 bg-white'>
        <For each={props.items}>
          {(item) => {
            const itemSubtitle = subtitle(item);

            return (
              <div
                classList={{
                  'cursor-pointer hover:bg-primary-hover': item.isDirectory,
                  'text-zinc-500': !item.isDirectory,
                }}
                class='flex flex-col gap-1 px-3 py-2 transition-colors'
                onClick={() => {
                  if (item.isDirectory) {
                    props.onClickDirectory?.(item);
                  }
                }}
              >
                <div class='flex items-center justify-between gap-3'>
                  <p class='truncate text-sm text-zinc-800' title={item.title}>
                    {item.title}
                  </p>
                  <span class='shrink-0 text-[10px] uppercase tracking-[0.2em] text-zinc-400'>{item.isDirectory ? 'dir' : 'file'}</span>
                </div>

                <Show when={itemSubtitle}>
                  <p class='truncate text-xs text-zinc-500' title={itemSubtitle}>
                    {itemSubtitle}
                  </p>
                </Show>
              </div>
            );
          }}
        </For>
      </div>
    </Show>
  );
};

export default MusicDirectoryList;
