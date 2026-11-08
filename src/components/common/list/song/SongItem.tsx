import { Component, Match, Show, Switch } from 'solid-js';
import type { SongResponse } from '~/bindings';
import type { MenuItem } from '~/features/menu/types';
import useContextMenu from '~/features/menu/useContextMenu';
import { formatCompactDuration } from '~/utils/duration';
import { orUnknown } from '~/utils/unknownValue';

import './SongItem.css';

export interface SongItemProps {
  song: SongResponse;
  /**
   * What the leading column shows: nothing (`hidden`, no column), `index` or ▶ while `playing`
   * (`indexOrPlaying`, the default), or a checkbox reflecting `selected` (`selection`).
   */
  indexContentMode?: 'hidden' | 'indexOrPlaying' | 'selection';
  index?: number;
  /** The playing look (background, text color, ▶ in place of the index). */
  playing?: boolean;
  /** The selected look (tint, bar at the left edge). */
  selected?: boolean;
  coverArt?: boolean;
  coverArtUrl?: string | null;
  /** When the title is bold; `playingOnly` by default. */
  boldTitle?: 'always' | 'playingOnly' | 'never';
  playNext?: boolean;
  withMenuButton?: boolean;
  onMenuButtonClick?: (event: MouseEvent) => void;
  contextMenuItems?: MenuItem[];
  onClick?: () => void;
}

/**
 * @description Album/Queue/Search で共有する曲一行コンポーネント
 */
const SongItem: Component<SongItemProps> = (props) => {
  const menuHandlers = props.contextMenuItems ? useContextMenu(props.contextMenuItems) : {};
  const boldTitle = () => {
    const mode = props.boldTitle ?? 'playingOnly';
    return mode === 'always' || (mode === 'playingOnly' && !!props.playing);
  };

  return (
    <div
      class='song-item ripple relative flex w-full cursor-pointer flex-row items-center overflow-x-hidden py-2'
      onClick={() => props.onClick?.()}
      // SongItem.css draws the playing and selected looks from these.
      data-current={props.playing ? 'true' : undefined}
      data-selected={props.selected ? 'true' : undefined}
      {...menuHandlers}
    >
      <Show when={props.indexContentMode !== 'hidden'} fallback={<div class='ml-4' />}>
        <div class='flex w-13 flex-row items-center justify-center'>
          <Switch>
            <Match when={props.indexContentMode === 'selection'}>
              {/* relative z-1: drawn in front of the selection tint (.song-item[data-selected]::after). */}
              <Show
                when={props.selected}
                fallback={<i class='icon-[material-symbols--check-box-outline-blank] song-item-leading relative z-1 text-xl' />}
              >
                <i class='icon-[material-symbols--check-box] song-item-leading relative z-1 text-xl' />
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
          <p class='song-item-title min-w-0 flex-1 truncate' classList={{ 'font-bold': boldTitle() }} title={props.song.title}>
            {orUnknown(props.song.title)}
          </p>
        </div>
        <p class='song-item-meta truncate text-[11px]' title={props.song.artist ?? undefined}>
          {orUnknown(props.song.artist)}
        </p>
      </div>

      <p class='song-item-meta ml-3 text-xs'>{formatCompactDuration(props.song.duration ?? 0)}</p>

      <Show when={props.withMenuButton} fallback={<div class='mr-4' />}>
        {/* Its presses stay with the button: they neither click the row nor start a press on it. */}
        <div
          role='button'
          aria-label='More'
          class='song-item-meta ripple ml-2 flex items-center justify-center self-stretch rounded-full p-2'
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation();
            props.onMenuButtonClick?.(event);
          }}
        >
          <i class='icon-[material-symbols--more-vert] text-lg' />
        </div>
      </Show>
    </div>
  );
};

export default SongItem;
