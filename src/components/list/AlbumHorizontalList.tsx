import { Component, For, Show } from 'solid-js';
import type { AlbumListItem } from '~/bindings';
import AlbumGridItem from './item/album/AlbumGridItem';

interface AlbumHorizontalListProps {
  albums: AlbumListItem[];
  emptyMessage?: string;
}

const AlbumHorizontalList: Component<AlbumHorizontalListProps> = (props) => {
  return (
    <Show when={props.albums.length > 0} fallback={<div class='px-2 py-6 text-sm text-zinc-400'>{props.emptyMessage ?? 'No albums available.'}</div>}>
      <div class='flex flex-row gap-2 overflow-x-auto py-2'>
        <For each={props.albums}>{(album) => <AlbumGridItem album={album} />}</For>
      </div>
    </Show>
  );
};

export default AlbumHorizontalList;
