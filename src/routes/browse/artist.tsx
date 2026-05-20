import { Icon } from '@iconify-icon/solid';
import { useParams, type RouteSectionProps } from '@solidjs/router';
import { createEffect, createMemo, createResource, createSignal, Match, Show, Switch } from 'solid-js';
import { commands, type ArtistInfo2Response, type ArtistResponse } from '~/bindings';
import Heading3 from '~/components/common/Heading3';
import AlbumGrid from '~/components/common/list/album/AlbumGrid';
import AlbumList from '~/components/common/list/album/AlbumList';
import { AlbumItem } from '~/components/common/list/album/item/AlbumGridItem';
import { type BrowseListItem } from '~/components/common/list/index/BrowseList';
import LoadCircle from '~/components/common/LoadCircle';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { fetchCoverArtAssetUrl } from '~/features/albums/service';
import { fetchArtistImageAssetUrl } from '~/features/artist/service';
import { resolveAlbumRoute, resolveArtistRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { sessionStore } from '~/stores/SessionStore';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';
import { setTempStore, tempStore } from '~/stores/TempStore';

const SIMILAR_ARTIST_COUNT = 8;

function BrowseArtist(props: Partial<RouteSectionProps<unknown>>) {
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
      href: resolveArtistRoute(similarArtist.id),
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

    return { type: 'coverArt' as const, profileId, coverArtId, size: CoverArtSizes.lg };
  });

  const [artistImage] = createResource(artistImageRequest, (req) => {
    if (req.type === 'external') {
      return fetchArtistImageAssetUrl(req.url);
    }
    return fetchCoverArtAssetUrl(req);
  });

  const [failedSrc, setFailedSrc] = createSignal<string | null>(null);
  const effectiveArtistImageSrc = createMemo(() => {
    if (artistImage.loading) return null;
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
    setFailedSrc(null);

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
    <div
      class='home-surface-root p-0'
      style={{
        'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
        'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
      }}
    >
      <Show when={error()}>{(message) => <p class='px-1 text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<LoadCircle class='m-3 self-center justify-self-center' />}>
        <Show when={artist()}>
          {(artistValue) => (
            <div class='flex flex-col'>
              <div class='relative z-10 flex h-42 w-full flex-col'>
                <div class='border-primary-border absolute top-0 right-0 bottom-0 left-0 h-full w-full border-b'>
                  <Show
                    when={effectiveArtistImageSrc()}
                    fallback={<div class='bg-primary-surface flex h-full w-full items-center justify-center text-xs'></div>}
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
                <div class='bg-primary-surface absolute top-0 right-0 bottom-0 left-0 flex h-full w-full opacity-75' />
                <p class='archivo text-primary-text absolute bottom-0 left-0 z-10 m-3 w-full text-2xl font-black tracking-tighter'>
                  {artistValue().name}
                </p>
              </div>

              <div class='z-20 flex flex-col'>
                <div class='border-secondary-border bg-primary-bg sticky top-0 left-0 z-10 flex w-full flex-row items-center border-b p-2'>
                  <Heading3 class='text-secondary-text'>albums</Heading3>
                  <div class='flex-1' />
                  <Icon
                    icon={tempStore.albumMode === 'grid' ? 'pixelarticons:grid-2x2-2' : 'pixelarticons:bulletlist-sharp'}
                    class='cursor-pointer p-0.5 text-[15px]'
                    onClick={() => {
                      setTempStore('albumMode', tempStore.albumMode === 'grid' ? 'list' : 'grid');
                    }}
                  />
                </div>
                <Switch>
                  <Match when={tempStore.albumMode === 'grid'}>
                    <div class='p-2'>
                      <AlbumGrid
                        albums={albums()}
                        inline={true}
                        noArtistName={true}
                        emptyMessage='No ID3 albums returned for this artist.'
                        onItemClick={(album) => {
                          navigate(resolveAlbumRoute(album.id));
                        }}
                      />
                    </div>
                  </Match>
                  <Match when={tempStore.albumMode === 'list'}>
                    <AlbumList
                      albums={albums()}
                      noArtistName={true}
                      emptyMessage='No ID3 albums returned for this artist.'
                      onItemClick={(album) => {
                        navigate(resolveAlbumRoute(album.id));
                      }}
                    />
                  </Match>
                </Switch>
              </div>
            </div>
          )}
        </Show>
      </Show>
    </div>
  );
}

export default BrowseArtist;
