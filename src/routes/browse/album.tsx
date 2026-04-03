import { useParams, type RouteSectionProps } from '@solidjs/router';
import { createEffect, createMemo, createResource, createSignal } from 'solid-js';
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

  return (
    <div class='home-surface-root p-0'>
      <div class='bg-primary-inner-surface relative flex w-full flex-col items-center p-5'>
        <div class='absolute top-0 right-0 bottom-0 left-0 flex h-full w-full shadow-[inset_0_-2px_8px_rgba(128,128,128,0.5)]' />

        <img class='border-secondary-border m-8 aspect-square h-54 w-54 border' src={coverArt() ?? undefined} />

        <div class='flex w-full flex-col items-start'>
          <MarqueeParagraph text={album()?.name || '[unknown]'} class='archivo-black w-full text-2xl' />
          <MarqueeParagraph class='archivo text-secondary-text' text={album()?.artist || '[unknown]'} />
        </div>
      </div>

      <div class='flex flex-col items-start'>
        <div class='border-secondary-border flex w-full flex-col items-start border-b p-2'>
          <Heading3>songs</Heading3>
        </div>
        <SongList songs={album()?.songs} />
      </div>
    </div>
  );
}

export default BrowseAlbum;
