import { Component, JSX, Match, Show, Switch } from 'solid-js';
import type { SongResponse } from '~/bindings';
import { type SongItemConditions, songItemConditionAttrs } from '~/features/items/SongItemConditions';
import type { MenuItem } from '~/features/menu/types';
import useContextMenu from '~/features/menu/useContextMenu';
import { formatCompactDuration } from '~/utils/duration';

export interface SongItemProps {
  song: SongResponse;
  conditions?: SongItemConditions;
  /** Whether to hide leading contents (index/playing/checkbox) */
  noIndexContent?: boolean;
  index?: number;
  playing?: boolean;
  /** Shows a checkbox instead of index or playing icon; no checkbox while undefined. */
  checked?: boolean;
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

  return (
    <div
      ref={props.ref}
      class={`song-item ripple relative flex w-full cursor-pointer flex-row items-center overflow-x-hidden py-2`}
      onClick={() => props.onClick?.()}
      {...songItemConditionAttrs(conditions())}
      {...menuHandlers}
    >
      {props.children}

      <Show when={!props.noIndexContent} fallback={<div class='ml-4' />}>
        <div class='flex w-13 flex-row items-center justify-center'>
          <Switch>
            <Match when={props.checked !== undefined}>
              <Show when={props.checked} fallback={<i class={'icon-[material-symbols--check-box-outline-blank] song-item-leading text-xl'} />}>
                <i class={'icon-[material-symbols--check-box] song-item-leading text-xl'} />
              </Show>
            </Match>
            <Match when={props.playing}>
              <i class='icon-[material-symbols--play-arrow] song-item-leading text-lg' />
            </Match>
            <Match when={props.index !== undefined}>
              <p class='song-item-leading text-sm font-bold'>{props.index}</p>
            </Match>
          </Switch>
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
          <p class='song-item-title min-w-0 flex-1 truncate' classList={{ 'font-bold': props.emphasizeTitle }} title={props.song.title}>
            {props.song.title}
          </p>
        </div>
        <Show when={props.song.artist}>
          <p class='song-item-meta truncate text-[11px]' title={props.song.artist ?? undefined}>
            {props.song.artist}
          </p>
        </Show>
      </div>

      <p class={`song-item-meta ml-3 text-xs`}>{formatCompactDuration(props.song.duration ?? 0)}</p>

      <Show when={props.withMenuButton} fallback={<div class='mr-4' />}>
        {/* Its presses stay with the button: they neither click the row nor start a press on it. */}
        <div
          role='button'
          aria-label='More'
          class='song-item-meta ripple ml-2 flex items-center justify-center self-stretch rounded-full p-2'
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
