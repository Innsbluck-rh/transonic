import { useParams, type RouteSectionProps } from '@solidjs/router';
import { createEffect, createMemo, createResource, createSignal, Show } from 'solid-js';
import { commands, type ArtistInfo2Response, type ArtistResponse } from '~/bindings';
import Heading2 from '~/components/common/Heading2';
import LoadCircle from '~/components/common/LoadCircle';
import AlbumGrid from '~/components/common/list/album/AlbumGrid';
import { AlbumItem } from '~/components/common/list/album/item/AlbumGridItem';
import { type BrowseListItem } from '~/components/common/list/index/BrowseList';
import { fetchCoverArtAssetUrl } from '~/features/albums/service';
import { fetchArtistImageAssetUrl } from '~/features/artist/service';
import { resolveAlbumRoute, resolveArtistRoute, type RouteVariant } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { sessionStore } from '~/stores/SessionStore';

const ARTIST_COVER_ART_SIZE = 320;
const SIMILAR_ARTIST_COUNT = 8;

interface BrowseArtistProps extends Partial<RouteSectionProps<unknown>> {
  routeVariant?: RouteVariant;
}

function BrowseArtist(props: BrowseArtistProps) {
  const params = useParams();
  const navigate = useSPNavigate();

  const [artist, setArtist] = createSignal<ArtistResponse | null>(null);
  const [artistInfo, setArtistInfo] = createSignal<ArtistInfo2Response | null>(null);
  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  let latestLoadId = 0;

  const albums = createMemo<AlbumItem[]>(() => artist()?.albums ?? []);

  const similarArtists = createMemo<BrowseListItem[]>(() =>
    (artistInfo()?.similarArtists ?? []).map((similarArtist) => ({
      id: similarArtist.id,
      name: similarArtist.name,
      href: resolveArtistRoute(similarArtist.id, props.routeVariant ?? 'desktop'),
    }))
  );

  const roleLabel = createMemo(() => {
    const roles = artist()?.roles ?? [];
    return roles.length > 0 ? roles.join(' / ') : null;
  });

  const externalImageUrl = createMemo(
    () => artistInfo()?.largeImageUrl ?? artistInfo()?.mediumImageUrl ?? artistInfo()?.smallImageUrl ?? artist()?.artistImageUrl ?? null
  );

  const artistImageRequest = createMemo(() => {
    const url = externalImageUrl();
    if (url) return { type: 'external' as const, url };

    const coverArtId = artist()?.coverArtId;
    const profileId = sessionStore.activeSession?.profileId;
    if (!coverArtId || !profileId) return null;

    return { type: 'coverArt' as const, profileId, coverArtId, size: ARTIST_COVER_ART_SIZE };
  });

  const [artistImage] = createResource(artistImageRequest, (req) => {
    if (req.type === 'external') {
      return fetchArtistImageAssetUrl(req.url);
    }
    return fetchCoverArtAssetUrl(req);
  });

  const [failedSrc, setFailedSrc] = createSignal<string | null>(null);
  const effectiveArtistImageSrc = createMemo(() => {
    const src = artistImage() ?? null;
    return src !== null && src === failedSrc() ? null : src;
  });

  async function loadArtistPage(rawArtistId: string) {
    const loadId = ++latestLoadId;

    if (!sessionStore.activeSession) {
      setArtist(null);
      setArtistInfo(null);
      setError(null);
      setIsLoading(false);
      return;
    }

    const artistId = decodeURIComponent(rawArtistId || '').trim();
    if (!artistId) {
      setArtist(null);
      setArtistInfo(null);
      setError('Artist id is missing.');
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const [artistResult, infoResult] = await Promise.all([
        commands.getArtist({ id: artistId }),
        commands.getArtistInfo2({
          id: artistId,
          count: SIMILAR_ARTIST_COUNT,
          includeNotPresent: false,
        }),
      ]);

      if (loadId !== latestLoadId) {
        return;
      }

      if (artistResult.status === 'error') {
        setArtist(null);
        setArtistInfo(null);
        setError(artistResult.error);
        return;
      }

      setArtist(artistResult.data);

      if (infoResult.status === 'error') {
        console.warn(infoResult.error);
        setArtistInfo(null);
      } else {
        setArtistInfo(infoResult.data);
      }
    } catch (invokeError) {
      if (loadId !== latestLoadId) {
        return;
      }

      console.error(invokeError);
      setArtist(null);
      setArtistInfo(null);
      setError('Failed to load artist details.');
    } finally {
      if (loadId === latestLoadId) {
        setIsLoading(false);
      }
    }
  }

  createEffect(() => {
    void sessionStore.activeSession;
    void loadArtistPage(params.id || '');
  });

  return (
    <div class='home-surface-root p-0'>
      <Show when={error()}>{(message) => <p class='px-1 text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<LoadCircle class='m-3 self-center justify-self-center' />}>
        <Show when={artist()}>
          {(artistValue) => (
            <div class='flex flex-col gap-2'>
              <div class='relative flex h-42 w-full flex-col'>
                <div class='border-primary-border absolute top-0 right-0 bottom-0 left-0 h-full w-full border-b'>
                  <Show
                    when={effectiveArtistImageSrc()}
                    fallback={<div class='bg-primary-inner-surface flex h-full w-full items-center justify-center text-xs'>No art</div>}
                  >
                    {(src) => (
                      <img
                        class='h-full w-full object-cover object-center'
                        src={src()}
                        onError={() => setFailedSrc(src())}
                        loading='lazy'
                        decoding='async'
                      />
                    )}
                  </Show>
                </div>
                <div class='bg-primary-inner-surface absolute top-0 right-0 bottom-0 left-0 flex h-full w-full opacity-75' />
                <div class='absolute top-0 right-0 bottom-0 left-0 flex h-full w-full shadow-[inset_0_-2px_8px_rgba(128,128,128,0.5)]' />
                <p class='archivo text-primary-text absolute bottom-0 left-0 z-10 m-4 w-full text-2xl font-black'>{artistValue().name}</p>
              </div>

              <div class='flex flex-col gap-2 p-3'>
                <Heading2>albums</Heading2>
                <AlbumGrid
                  albums={albums()}
                  emptyMessage='No ID3 albums returned for this artist.'
                  onItemClick={(album) => {
                    navigate(resolveAlbumRoute(album.id, props.routeVariant ?? 'desktop'));
                  }}
                />
              </div>
            </div>
          )}
        </Show>
      </Show>
    </div>
  );
}

export default BrowseArtist;
