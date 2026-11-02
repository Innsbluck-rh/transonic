import { useLocation } from '@solidjs/router';
import { Component, createEffect, on, Show } from 'solid-js';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import LoadCircle from '../LoadCircle';
import ZoomableImage from '../ZoomableImage';
import { albumArtOverlayCoverArtId, albumArtOverlayDialogOpen, closeAlbumArtOverlayDialog } from './service';

export const AlbumArtOverlayDialog: Component = () => {
  const location = useLocation();
  const close = () => {
    closeAlbumArtOverlayDialog();
  };

  createEffect(on(() => location.pathname, close, { defer: true }));

  const { src: coverArt, loading: _coverArtLoading } = useCoverArt(albumArtOverlayCoverArtId, CoverArtSizes.full); // load raw cover art

  return (
    <Show when={albumArtOverlayDialogOpen()}>
      <div class='absolute inset-0 z-100 flex items-center justify-center bg-black/75' role='presentation'>
        <div class='absolute top-5 right-5 z-10 cursor-pointer p-2 drop-shadow-[0_1px_8px_rgb(0_0_0)]' onClick={close}>
          <i class='icon-[material-symbols--close] text-3xl' />
        </div>

        <section class='flex size-full flex-col items-center justify-center overflow-hidden' role='dialog' aria-modal='true' onClick={close}>
          <div class='sp:h-[90%] sp:w-[90%] flex h-[50%] w-[50%] items-center justify-center shadow-lg'>
            <Show when={coverArt()} fallback={<LoadCircle />}>
              {(assetUrl) => <ZoomableImage src={assetUrl()} />}
            </Show>
          </div>
        </section>
      </div>
    </Show>
  );
};
