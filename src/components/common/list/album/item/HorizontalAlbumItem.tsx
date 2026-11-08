import { Component, Show } from 'solid-js';
import { AlbumListItem } from '~/bindings';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { buildAlbumMenuItems } from '~/features/menu';
import useContextMenu from '~/features/menu/useContextMenu';
import { resolveAlbumRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { orUnknown } from '~/utils/unknownValue';

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
      class='bg-primary-surface hover:bg-primary-hover ripple flex w-full cursor-pointer items-center gap-0 overflow-hidden rounded-md shadow'
      onClick={() => {
        navigate(resolveAlbumRoute(props.album.id));
      }}
      {...contextMenuProps}
    >
      <div class='bg-primary-surface border-secondary-border size-14 overflow-hidden border-r'>
        <Show when={!loading()} fallback={<div class='size-full' />}>
          <Show
            when={src()}
            fallback={
              <div class='flex size-full items-center justify-center bg-black/5'>
                <p class='text-secondary-text text-center text-[10px] font-bold'>no img</p>
              </div>
            }
          >
            {(assetUrl) => <img src={assetUrl()} class='size-full object-cover' />}
          </Show>
        </Show>
      </div>

      <div class='flex flex-1 flex-col justify-center overflow-hidden px-3'>
        <p class='archivo truncate font-bold'>{orUnknown(props.album.name)}</p>
        <p class='archivo text-secondary-text truncate text-xs'>{orUnknown(props.album.artist)}</p>
      </div>
    </div>
  );
};

export default HorizontalAlbumItem;
