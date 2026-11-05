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
  /** Shows a checkbox in place of `leadingContent`; no checkbox while undefined. */
  checked?: boolean;
  /** A ⋮ button after the duration; `onMenuClick` receives its clicks. */
  withMenuButton?: boolean;
  onMenuClick?: (event: MouseEvent) => void;
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
      class={`song-item ripple relative flex w-full cursor-pointer flex-row items-center overflow-x-hidden py-2 ${props.withMenuButton ? 'pr-1 pl-4' : 'px-4'}`}
      onClick={() => props.onClick?.()}
      {...songItemConditionAttrs(conditions())}
      {...menuHandlers}
    >
      {props.children}

      <Show when={props.checked !== undefined || leadingContent() != null}>
        <div class='flex w-8 flex-row items-start'>
          <Show
            when={props.checked === undefined}
            fallback={
              <i
                class={`${props.checked ? 'icon-[material-symbols--check-box]' : 'icon-[material-symbols--check-box-outline-blank]'} song-item-leading -ml-[3px] text-[16px]`}
              />
            }
          >
            {typeof leadingContent() === 'string' ? <p class='song-item-leading archivo text-xs font-bold'>{leadingContent()}</p> : leadingContent()}
          </Show>
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
            <i class='icon-[material-symbols--next-plan] text-secondary-text text-xs' />
          </Show>
          <p class='song-item-title archivo min-w-0 flex-1 truncate' classList={{ 'font-bold': props.emphasizeTitle }} title={props.song.title}>
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

      <Show when={props.withMenuButton}>
        {/* Its presses stay with the button: they neither click the row nor start a press on it. */}
        <div
          role='button'
          aria-label='More'
          data-ripple='off'
          class='song-item-meta ml-1 flex items-center self-stretch px-2'
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation();
            props.onMenuClick?.(event);
          }}
        >
          <i class='icon-[material-symbols--more-vert] text-lg' />
        </div>
      </Show>
    </div>
  );
};

export default SongItem;
