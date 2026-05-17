import { Component, Show } from 'solid-js';
import type { AlbumListItem, ArtistAlbum, FolderStructureAlbumItem } from '~/bindings';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { buildAlbumMenuItems } from '~/features/menu';
import useContextMenu from '~/features/menu/useContextMenu';
import { usePlayback } from '~/features/playback/usePlayback';

export type AlbumItem = AlbumListItem | ArtistAlbum | FolderStructureAlbumItem;

interface AlbumListItemProps {
  class?: string;
  album: AlbumItem;
  noArtistName?: boolean;
  onClick?: (e: MouseEvent) => void;
  onDblClick?: (e: MouseEvent) => void;
}

const AlbumListItem: Component<AlbumListItemProps> = (props) => {
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
  const isClickable = !!(props.onClick || props.onDblClick);
  const { src, loading } = useCoverArt(() => props.album.coverArtId ?? null, CoverArtSizes.sm, {
    cachedFallbackSizes: [CoverArtSizes.lg, CoverArtSizes.md],
  });

  return (
    <div
      class={`ripple group hover:bg-primary-hover flex shrink-0 flex-row items-center rounded-md px-2 py-2 ${props.class}`}
      {...contextMenuProps}
      classList={{
        'cursor-pointer': isClickable,
      }}
      onClick={(e) => props.onClick?.(e)}
      onDblClick={(e) => props.onDblClick?.(e)}
    >
      <Show when={src()} fallback={<div class='border-secondary-border mr-3 aspect-square max-h-9 w-9 rounded-md border' />}>
        {(assetUrl) => <img src={assetUrl()} class='mr-3 aspect-square max-h-9 w-9 rounded-md' loading='lazy' decoding='async' />}
      </Show>

      <div class='flex min-w-0 flex-1 flex-col justify-center'>
        <p class='text-md truncate font-bold' title={props.album.name ?? '[unknown]'}>
          {props.album.name}
        </p>
        <Show when={!props.noArtistName}>
          <p class='text-secondary-text truncate text-[10px]' title={props.album.artist ?? undefined}>
            {props.album.artist ?? 'unknown artist'}
          </p>
        </Show>
      </div>
      <p class='text-secondary-text mx-1 truncate text-xs'>{props.album.year}</p>
    </div>
  );
};

export default AlbumListItem;
