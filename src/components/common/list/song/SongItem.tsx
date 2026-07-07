import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, JSX, Show } from 'solid-js';
import type { SongResponse } from '~/bindings';
import { type SongItemConditions, songItemConditionAttrs } from '~/features/items/SongItemConditions';
import type { MenuItem } from '~/features/menu/types';
import useContextMenu from '~/features/menu/useContextMenu';
import { formatCompactDuration } from '~/utils/duration';

export interface SongItemProps {
  song: SongResponse;
  conditions?: SongItemConditions;
  leadingContent?: string | JSX.Element;
  coverArt?: boolean;
  coverArtUrl?: string | null;
  emphasizeTitle?: boolean;
  playNext?: boolean;
  menuItems?: MenuItem[];
  onClick?: () => void;
  ref?: (element: HTMLDivElement) => void;
  children?: JSX.Element;
}

/**
 * @description Album/Queue/Search で共有する曲一行コンポーネント
 */
const SongItem: Component<SongItemProps> = (props) => {
  const menuHandlers = props.menuItems ? useContextMenu(props.menuItems) : {};

  const conditions = (): SongItemConditions => props.conditions ?? { current: false, selected: false };
  const leadingContent = createMemo(() => props.leadingContent);

  return (
    <div
      ref={props.ref}
      class='song-item ripple relative flex w-full cursor-pointer flex-row items-center overflow-x-hidden px-4 py-2'
      onClick={() => props.onClick?.()}
      {...songItemConditionAttrs(conditions())}
      {...menuHandlers}
    >
      {props.children}

      <Show when={leadingContent() != null}>
        <div class='flex w-8 flex-row items-start'>
          {typeof leadingContent() === 'string' ? <p class='song-item-leading archivo text-xs font-bold'>{leadingContent()}</p> : leadingContent()}
        </div>
      </Show>

      <Show when={props.coverArt}>
        <Show when={props.coverArtUrl} fallback={<div class='border-secondary-border mr-3 aspect-square max-h-8 w-8 rounded-md border' />}>
          {(url) => <img src={url()} class='mr-3 aspect-square max-h-8 w-8 rounded-md' loading='lazy' decoding='async' />}
        </Show>
      </Show>

      <div class='flex min-w-0 flex-1 flex-col gap-0'>
        <div class='flex items-center gap-1'>
          <Show when={props.playNext}>
            <Icon class='text-secondary-text text-xs' icon='material-symbols:next-plan' />
          </Show>
          <p class='song-item-title text-md min-w-0 flex-1 truncate' classList={{ 'font-bold': props.emphasizeTitle }} title={props.song.title}>
            {props.song.title}
          </p>
        </div>
        <Show when={props.song.artist}>
          <p class='song-item-meta archivo truncate text-[11px]' title={props.song.artist ?? undefined}>
            {props.song.artist}
          </p>
        </Show>
      </div>

      <p class='song-item-meta archivo ml-2 text-xs'>{formatCompactDuration(props.song.duration ?? 0)}</p>
    </div>
  );
};

export default SongItem;
