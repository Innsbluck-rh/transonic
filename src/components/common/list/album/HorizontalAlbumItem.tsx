import { Component, Show } from 'solid-js';
import { AlbumListItem } from '~/bindings';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { buildAlbumMenuItems } from '~/features/menu';
import useContextMenu from '~/features/menu/useContextMenu';
import { resolveAlbumRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';

interface HorizontalAlbumItemProps {
  album: AlbumListItem;
}

const HorizontalAlbumItem: Component<HorizontalAlbumItemProps> = (props) => {
  const navigate = useSPNavigate();
  const { src, loading } = useCoverArt(() => props.album.coverArtId ?? null, CoverArtSizes.md, {
    cachedFallbackSizes: [CoverArtSizes.lg],
  });
  const contextMenuProps = useContextMenu(
    buildAlbumMenuItems({
      albumId: props.album.id,
      type: 'album',
    })
  );

  return (
    <div
      class='bg-primary-surface hover:bg-primary-hover ripple flex w-full cursor-pointer items-center gap-0 overflow-hidden rounded-md'
      onClick={() => {
        navigate(resolveAlbumRoute(props.album.id));
      }}
      {...contextMenuProps}
    >
      <div class='size-14 overflow-hidden md:size-12 lg:size-14'>
        <Show when={src()} fallback={<div class='size-full' />}>
          {(assetUrl) => <img src={assetUrl()} class='size-full object-cover' />}
        </Show>
      </div>

      <div class='flex flex-1 flex-col justify-center overflow-hidden px-3'>
        <p class='archivo text-md truncate font-bold'>{props.album.name}</p>
        <p class='archivo text-secondary-text truncate text-xs'>{props.album.artist}</p>
      </div>
    </div>
  );
};

export default HorizontalAlbumItem;
