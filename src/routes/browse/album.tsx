import { useParams, type RouteSectionProps } from '@solidjs/router';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import { commands, type AlbumSongsResponse } from '~/bindings';
import { openAlbumArtOverlayDialog } from '~/components/common/dialog/service';
import AlbumTrackList from '~/components/common/list/song/AlbumTrackList';
import LoadCircle from '~/components/common/LoadCircle';
import Heading3 from '~/components/common/text/Heading3';
import MarqueeParagraph from '~/components/common/text/MarqueeParagraph';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArt } from '~/features/albums/useCoverArt';
import { buildAlbumMenuItems } from '~/features/menu';
import useContextMenu from '~/features/menu/useContextMenu';
import { resolveArtistRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { usePlayback } from '~/features/playback/usePlayback';
import { sessionStore } from '~/stores/SessionStore';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';
import { isSP } from '~/utils/isSP';
import { orUnknown } from '~/utils/unknownValue';

function BrowseAlbum(props: Partial<RouteSectionProps<unknown>>) {
  const navigate = useSPNavigate();
  const params = useParams();
  const { playAlbum, insertAfterCurrent, appendToQueue } = usePlayback();

  const [album, setAlbum] = createSignal<AlbumSongsResponse | null>(null);
  const contextMenuProps = createMemo(() => {
    const albumId = album()?.id;
    if (!albumId) return {};
    return useContextMenu(
      buildAlbumMenuItems({
        albumId,
        type: 'album',
      })
    );
  });
  let latestLoadId = 0;

  const { src: coverArt, loading: coverArtLoading } = useCoverArt(() => album()?.coverArtId ?? null, CoverArtSizes.lg);

  const loadAlbum = async (rawAlbumId: string) => {
    const loadId = ++latestLoadId;

    const albumId = decodeURIComponent(rawAlbumId || '').trim();
    if (!albumId) {
      setAlbum(null);
      return;
    }

    try {
      const result = await commands.getAlbumInfo({ id: albumId });
      if (loadId !== latestLoadId) return;

      if (result.status === 'ok') {
        setAlbum(result.data);
      } else {
        console.error(result.error);
        setAlbum(null);
      }
    } catch (err) {
      if (loadId !== latestLoadId) return;
      console.error(err);
      setAlbum(null);
    }
  };

  createEffect(() => {
    void sessionStore.activeSession;
    void loadAlbum(params.id || '');
  });

  const infoText = createMemo<string | null>(() => {
    if (album() === null) return null;
    const songCount = album()?.songCount;
    const infoText = [album()?.year, album()?.genre, songCount != null ? `${songCount} songs` : null]
      .map((value) => {
        if (!value) return null;
        return String(value);
      })
      .filter((str) => str?.trim())
      .join('・');
    if (infoText.trim()) return infoText;
    else return null;
  });

  return (
    <div class='home-surface-root w-full gap-0 p-0'>
      <div class='bg-primary-surface flex w-full items-center gap-1 p-2'>
        <div
          class='ripple hover:bg-primary-hover sp:text-2xl mr-auto flex cursor-pointer rounded-full p-1 text-xl'
          onClick={() => {
            navigate(-1);
          }}
        >
          <i class='icon-[material-symbols--arrow-back]' />
        </div>
      </div>

      <div class='bg-primary-surface sp:flex-col sp:pb-4 sp:pt-0 relative z-10 flex w-full flex-row items-center px-3.5 pt-1 pb-5'>
        {/* fit cover div in square space */}
        <div class='sp:w-52 sp:h-52 flex h-48 w-48 items-center justify-center'>
          <div
            class='group border-secondary-border sp:w-52 pointer-events-auto relative z-10 w-48 border shadow-lg'
            onClick={() => {
              const coverArtId = album()?.coverArtId;
              if (coverArtId) openAlbumArtOverlayDialog(coverArtId);
            }}
            {...contextMenuProps()}
          >
            <Show
              when={coverArt()}
              fallback={
                <div class='sp:w-52 flex aspect-square w-48 items-center justify-center'>
                  <LoadCircle />
                </div>
              }
            >
              {(assetUrl) => <img class='sp:w-52 w-48 cursor-pointer hover:brightness-75' src={assetUrl()} loading='lazy' decoding='async' />}
            </Show>
          </div>
        </div>

        <div class='sp:mt-6 sp:h-auto sp:min-w-auto sp:flex-initial sp:px-0 flex h-full w-full min-w-0 flex-1 flex-col items-start px-5'>
          <MarqueeParagraph
            text={album() ? orUnknown(album()?.name) : ''}
            class='archivo sp:mb-0 sp:text-2xl mb-1 w-full text-3xl font-black tracking-tighter'
          />
          <MarqueeParagraph
            onClick={() => {
              const artistId = album()?.artistId;
              if (artistId) navigate(resolveArtistRoute(artistId));
            }}
            class='archivo text-secondary-text hover:text-accent mt-0.5 cursor-pointer'
            text={album() ? orUnknown(album()?.artist) : ''}
          />
          <Show when={infoText()}>
            <p class='archivo text-secondary-text mt-1.5 text-[12px]'>{infoText()}</p>
          </Show>

          <Show when={!isSP()} fallback={<div class='mt-2' />}>
            <div class='mt-auto mb-1 flex flex-row flex-wrap gap-3'>
              <button
                class='button-borderless text-accent bg-accent/10 hover:bg-accent/20 flex items-center gap-1.5'
                onClick={() => {
                  const albumId = album()?.id;
                  if (albumId) playAlbum(albumId);
                }}
              >
                <i class='icon-[material-symbols--play-arrow] text-accent text-lg' />
                play
              </button>
              <div class='mt-auto flex flex-row gap-1'>
                <button
                  class='button-borderless text-secondary-text flex items-center gap-1.5'
                  onClick={() => {
                    const albumId = album()?.id;
                    if (albumId) insertAfterCurrent([{ type: 'album', albumId }]);
                  }}
                >
                  <i class='icon-[material-symbols--next-plan] text-primary-text text-lg' />
                  play next
                </button>
                <button
                  class='button-borderless text-secondary-text flex items-center gap-1.5'
                  onClick={() => {
                    const albumId = album()?.id;
                    if (albumId) appendToQueue([{ type: 'album', albumId }]);
                  }}
                >
                  <i class='icon-[material-symbols--playlist-add] text-primary-text text-lg' />
                  add to queue
                </button>
              </div>
            </div>
          </Show>
        </div>
      </div>

      <div class='border-secondary-border z-20 flex w-full flex-col border-t pb-2'>
        <div class='border-secondary-border bg-primary-bg sticky top-0 left-0 z-10 flex w-full flex-row items-center border-b px-2 py-1.5'>
          <Heading3 class='text-secondary-text'>songs</Heading3>
        </div>
        <div
          style={{
            'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
            'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
          }}
        >
          <AlbumTrackList songs={album()?.songs} />
        </div>
      </div>
    </div>
  );
}

export default BrowseAlbum;
