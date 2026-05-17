import { Icon } from '@iconify-icon/solid';
import { useParams, type RouteSectionProps } from '@solidjs/router';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import { commands, type AlbumSongsResponse } from '~/bindings';
import Heading3 from '~/components/common/Heading3';
import SongList from '~/components/common/list/song/SongList';
import LoadCircle from '~/components/common/LoadCircle';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
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

function BrowseAlbum(props: Partial<RouteSectionProps<unknown>>) {
  const navigate = useSPNavigate();
  const params = useParams();
  const { playAlbum, insertAfterCurrent, appendToQueue } = usePlayback();

  const [album, setAlbum] = createSignal<AlbumSongsResponse | null>(null);
  const contextMenuProps = createMemo(() => {
    const albumId = album()?.id;
    if (!albumId) return {};
    return useContextMenu(
      buildAlbumMenuItems(
        {
          albumId,
          type: 'album',
        },
        { insertAfterCurrent, appendToQueue }
      )
    );
  });
  let latestLoadId = 0;

  const { src: coverArt, loading: coverArtLoading } = useCoverArt(() => album()?.coverArtId ?? null, CoverArtSizes.lg);

  const loadAlbum = async (rawAlbumId: string) => {
    const loadId = ++latestLoadId;

    if (!sessionStore.activeSession) {
      setAlbum(null);
      return;
    }

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
    const infoText = [album()?.year, album()?.genre, `${album()?.songCount} songs`]
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
      <div class='bg-primary-inner-surface relative z-10 flex w-full flex-col items-center px-3.5 pt-5 pb-0 lg:flex-row lg:pt-4 lg:pb-4'>
        <div
          class='group border-secondary-border pointer-events-auto relative z-10 h-auto w-52 max-w-1/2 border shadow-xl lg:w-42'
          onClick={() => {
            const albumId = album()?.id;
            if (albumId) playAlbum(albumId);
          }}
          {...contextMenuProps()}
        >
          <Show
            when={coverArt()}
            fallback={
              <div class='flex aspect-square w-52 items-center justify-center lg:w-42'>
                <LoadCircle />
              </div>
            }
          >
            {(assetUrl) => <img class='w-52 cursor-pointer hover:brightness-75 lg:w-42' src={assetUrl()} loading='lazy' decoding='async' />}
          </Show>
          <Icon
            class='pointer-events-none collapse absolute top-1/2 left-1/2 -translate-1/2 scale-300 text-white group-hover:visible'
            icon='material-symbols:play-arrow'
          />
        </div>

        <div class='mt-4 flex w-full flex-col items-start lg:mt-4 lg:h-full lg:min-w-0 lg:flex-1 lg:px-5 lg:py-0'>
          <MarqueeParagraph text={album()?.name || '[unknown]'} class='archivo text-2xl font-black tracking-tighter lg:mb-1 lg:text-3xl' />
          <MarqueeParagraph
            onClick={() => {
              const artistId = album()?.artistId;
              if (artistId) navigate(resolveArtistRoute(artistId));
            }}
            class='archivo text-secondary-text hover:text-accent mt-0.5 cursor-pointer'
            text={album()?.artist || '[unknown]'}
          />
          <Show when={infoText()}>
            <p class='archivo text-secondary-text mt-1.5 text-[12px]'>{infoText()}</p>
          </Show>

          <Show when={!isSP()} fallback={<div class='mt-4' />}>
            <div class='mt-3 mb-3 flex flex-row gap-2 lg:mt-auto'>
              <button
                class='button-borderless flex items-center gap-1.5'
                onClick={() => {
                  const albumId = album()?.id;
                  if (albumId) insertAfterCurrent([{ type: 'album', albumId }]);
                }}
              >
                <Icon icon='material-symbols:next-plan' />
                play next
              </button>
              <button
                class='button-borderless flex items-center gap-1.5'
                onClick={() => {
                  const albumId = album()?.id;
                  if (albumId) appendToQueue([{ type: 'album', albumId }]);
                }}
              >
                <Icon icon='material-symbols:playlist-add' />
                add to queue
              </button>
            </div>
          </Show>
        </div>
      </div>

      <div class='bg-primary-surface border-secondary-border z-20 flex w-full flex-col border-t pb-4'>
        <div class='border-secondary-border bg-primary-surface sticky top-0 left-0 z-10 flex w-full flex-row items-center border-b p-2'>
          <Heading3 class='text-secondary-text'>songs</Heading3>
        </div>
        <div
          style={{
            'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
            'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
          }}
        >
          <SongList songs={album()?.songs} />
        </div>
      </div>
    </div>
  );
}

export default BrowseAlbum;
