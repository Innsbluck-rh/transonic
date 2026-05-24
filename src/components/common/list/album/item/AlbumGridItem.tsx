import { Component, Show } from 'solid-js';
import type { AlbumListItem, ArtistAlbum, FolderStructureAlbumItem } from '~/bindings';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { buildAlbumMenuItems } from '~/features/menu';
import useContextMenu from '~/features/menu/useContextMenu';
import { usePlayback } from '~/features/playback/usePlayback';

export type AlbumItem = AlbumListItem | ArtistAlbum | FolderStructureAlbumItem;

interface AlbumGridItemProps {
  class?: string;
  album: AlbumItem;
  noArtistName?: boolean;
  inline?: boolean;
  onClick?: (e: MouseEvent) => void;
  onDblClick?: (e: MouseEvent) => void;
  onArtClick?: (e: MouseEvent) => void;
  onArtDblClick?: (e: MouseEvent) => void;
}

const AlbumGridItem: Component<AlbumGridItemProps> = (props) => {
  const { insertAfterCurrent, appendToQueue } = usePlayback();
  const contextMenuProps = useContextMenu(
    buildAlbumMenuItems(
      {
        albumId: props.album.id,
        type: 'album',
      },
      { insertAfterCurrent, appendToQueue }
    )
  );
  const isClickable = !!(props.onClick || props.onDblClick || props.onArtClick || props.onArtDblClick);

  const { src, loading } = useCoverArt(() => props.album.coverArtId ?? null, CoverArtSizes.md, {
    cachedFallbackSizes: [CoverArtSizes.lg],
  });

  return (
    <div
      class={`group flex shrink-0 flex-col ${props.class}`}
      onClick={(e) => props.onArtClick?.(e)}
      onDblClick={(e) => props.onArtDblClick?.(e)}
      {...contextMenuProps}
      classList={{
        'cursor-pointer': isClickable,
      }}
    >
      <div
        class='ripple border-secondary-border relative aspect-square size-full border'
        classList={{
          'cursor-pointer': isClickable,
        }}
        onClick={(e) => props.onClick?.(e)}
        onDblClick={(e) => props.onDblClick?.(e)}
      >
        <Show when={src()} fallback={<div class='border-secondary-border size-full border' />}>
          {(assetUrl) => <img src={assetUrl()} class='size-full object-cover' loading='lazy' decoding='async' />}
        </Show>
        <Show when={props.inline}>
          <div class='absolute right-1 bottom-1 left-1 flex flex-col gap-0.5 overflow-hidden'>
            <p class='bg-primary-surface/60 inline w-fit rounded px-1 py-0.5 text-xs font-bold' title={props.album.name ?? '[unknown]'}>
              {props.album.name}
            </p>
            <Show when={!props.noArtistName}>
              <span
                class='bg-primary-surface/60 inline w-fit truncate rounded px-1 py-0.5 text-[10px] leading-3'
                title={props.album.artist ?? undefined}
              >
                {props.album.artist ?? 'unknown artist'}
              </span>
            </Show>
          </div>
        </Show>
      </div>
      <Show when={!props.inline}>
        <div class='mt-1 flex flex-col'>
          <p class='text-md truncate font-bold' title={props.album.name ?? '[unknown]'}>
            {props.album.name}
          </p>
          <Show when={!props.noArtistName}>
            <p class='text-secondary-text truncate text-xs' title={props.album.artist ?? undefined}>
              {props.album.artist ?? 'unknown artist'}
            </p>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default AlbumGridItem;
