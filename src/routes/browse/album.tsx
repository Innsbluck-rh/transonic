import { Icon } from '@iconify-icon/solid';
import { useParams, type RouteSectionProps } from '@solidjs/router';
import { createEffect, createMemo, createResource, createSignal, Show } from 'solid-js';
import { commands, type AlbumSongsResponse } from '~/bindings';
import Heading3 from '~/components/common/Heading3';
import SongList from '~/components/common/list/song/SongList';
import MarqueeParagraph from '~/components/common/MarqueeParagraph';
import { fetchCoverArtAssetUrl } from '~/features/albums/service';
import { type RouteVariant } from '~/features/navigation/routes';
import { usePlayback } from '~/features/playback/usePlayback';
import { sessionStore } from '~/stores/SessionStore';

const ALBUM_COVER_ART_SIZE = 320;

interface BrowseAlbumProps extends Partial<RouteSectionProps<unknown>> {
  routeVariant?: RouteVariant;
}

function BrowseAlbum(props: BrowseAlbumProps) {
  const params = useParams();
  const { playAlbum } = usePlayback();

  const [album, setAlbum] = createSignal<AlbumSongsResponse | null>(null);
  let latestLoadId = 0;

  const coverArtRequest = createMemo(() => {
    const coverArtId = album()?.coverArtId;
    const profileId = sessionStore.activeSession?.profileId;
    if (!coverArtId || !profileId) return null;
    return { profileId, coverArtId, size: ALBUM_COVER_ART_SIZE };
  });

  const [coverArt] = createResource(coverArtRequest, fetchCoverArtAssetUrl);

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
      <div class='bg-primary-inner-surface relative flex w-full flex-col items-center px-3.5 pt-6 pb-4 lg:flex-row lg:pt-4'>
        <div class='absolute top-0 right-0 bottom-0 left-0 flex h-full w-full shadow-[inset_0_-2px_8px_rgba(128,128,128,0.5)]' />

        <div class='group border-secondary-border relative z-10 h-auto w-52 max-w-1/2 border lg:w-42'>
          <img
            class='w-52 cursor-pointer hover:brightness-75 lg:w-42'
            src={coverArt() ?? undefined}
            onClick={() => {
              const albumId = album()?.id;
              if (albumId) playAlbum(albumId);
            }}
          />
          <Icon class='collapse absolute top-1/2 left-1/2 -translate-1/2 scale-300 group-hover:visible' icon='material-symbols:play-arrow' />
        </div>

        <div class='mt-8 flex w-full flex-col items-start lg:mt-1 lg:h-full lg:min-w-0 lg:flex-1 lg:px-5 lg:py-0'>
          <MarqueeParagraph text={album()?.name || '[unknown]'} class='archivo text-2xl font-black tracking-tighter lg:mb-1 lg:text-3xl' />
          <MarqueeParagraph class='archivo text-secondary-text mt-1' text={album()?.artist || '[unknown]'} />
          <Show when={infoText()}>
            <p class='archivo text-secondary-text mt-1.5 text-[12px]'>{infoText()}</p>
          </Show>
        </div>
      </div>

      <div class='flex w-full flex-col'>
        <div class='border-secondary-border flex w-full flex-row items-center border-b p-2'>
          <Heading3>songs</Heading3>
        </div>
        <SongList songs={album()?.songs} />
      </div>
    </div>
  );
}

export default BrowseAlbum;
