import { useParams, type RouteSectionProps } from '@solidjs/router';
import { createEffect, createMemo, createResource, createSignal, Match, Show, Switch } from 'solid-js';
import { commands, type ArtistInfo2Response, type ArtistResponse } from '~/bindings';
import AlbumGrid from '~/components/common/list/album/AlbumGrid';
import AlbumList from '~/components/common/list/album/AlbumList';
import LoadCircle from '~/components/common/LoadCircle';
import Heading3 from '~/components/common/text/Heading3';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { fetchCoverArtAssetUrl } from '~/features/albums/service';
import { fetchArtistImageAssetUrl } from '~/features/artist/service';
import { resolveAlbumRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import { setAppearanceSetting, settingsStore } from '~/features/settings/service';
import { sessionStore } from '~/stores/SessionStore';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';
import { orUnknown } from '~/utils/unknownValue';

const SIMILAR_ARTIST_COUNT = 8;

type ArtistImageRequest = { type: 'external'; url: string } | { type: 'coverArt'; profileId: string; coverArtId: string; size: number };

type ArtistImageResourceValue = {
  requestKey: string;
  src: string | null;
};

function artistImageRequestKey(request: ArtistImageRequest): string {
  return request.type === 'external'
    ? ['external', request.url].join('\0')
    : ['coverArt', request.profileId, request.coverArtId, request.size].join('\0');
}

function BrowseArtist(props: Partial<RouteSectionProps<unknown>>) {
  const params = useParams();
  const navigate = useSPNavigate();

  const [artist, setArtist] = createSignal<ArtistResponse | null>(null);
  const [artistInfo, setArtistInfo] = createSignal<ArtistInfo2Response | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  let latestLoadId = 0;

  const externalImageUrl = createMemo(
    () => artistInfo()?.largeImageUrl ?? artistInfo()?.mediumImageUrl ?? artistInfo()?.smallImageUrl ?? artist()?.artistImageUrl ?? null
  );

  const artistImageRequest = createMemo((): ArtistImageRequest | null => {
    const url = externalImageUrl();
    if (url) return { type: 'external', url };

    const coverArtId = artist()?.coverArtId;
    const profileId = sessionStore.activeSession?.profileId;
    if (!coverArtId || !profileId) return null;

    return { type: 'coverArt', profileId, coverArtId, size: CoverArtSizes.lg };
  });
  const requestKey = createMemo(() => {
    const req = artistImageRequest();
    return req ? artistImageRequestKey(req) : null;
  });

  // A null source leaves the previous value in the resource, so the value carries its request key.
  const [artistImage] = createResource(requestKey, async (key): Promise<ArtistImageResourceValue | undefined> => {
    const req = artistImageRequest();
    if (!req || artistImageRequestKey(req) !== key) {
      return undefined;
    }

    return {
      requestKey: key,
      src: req.type === 'external' ? await fetchArtistImageAssetUrl(req.url) : await fetchCoverArtAssetUrl(req),
    };
  });

  const [failedSrc, setFailedSrc] = createSignal<string | null>(null);
  const effectiveArtistImageSrc = createMemo(() => {
    const key = requestKey();
    if (!key || artistImage.loading) return null;
    const image = artistImage();
    if (!image || image.requestKey !== key) return null;
    return image.src !== null && image.src === failedSrc() ? null : image.src;
  });

  async function loadArtistPage(rawArtistId: string) {
    const loadId = ++latestLoadId;

    const artistId = decodeURIComponent(rawArtistId || '').trim();
    if (!artistId) {
      setArtist(null);
      setArtistInfo(null);
      setError('Artist id is missing.');
      return;
    }

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

      <div class='flex flex-col'>
        <div class='relative z-20 flex h-42 w-full flex-col'>
          <div class='border-primary-border absolute inset-0 size-full border-b'>
            <Show
              when={effectiveArtistImageSrc()}
              fallback={<div class='bg-primary-surface flex size-full items-center justify-center text-xs'></div>}
            >
              {(src) => (
                <img class='size-full object-cover object-center' src={src()} onError={() => setFailedSrc(src())} loading='lazy' decoding='async' />
              )}
            </Show>
          </div>
          <div class='bg-primary-surface absolute inset-0 size-full opacity-75' />
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
          <p class='archivo text-primary-text absolute bottom-0 left-0 m-3 w-full text-2xl font-black tracking-tighter'>
            {artist() ? orUnknown(artist()?.name) : ''}
          </p>
        </div>

        <div class='flex flex-col'>
          <div class='border-secondary-border bg-primary-bg sticky top-0 left-0 z-10 flex w-full flex-row items-center border-b p-2'>
            <Heading3 class='text-secondary-text'>albums</Heading3>
            <div class='flex-1' />
            <div
              class='flex cursor-pointer items-center gap-0.5 px-0.5'
              onClick={() => {
                void setAppearanceSetting('albumDisplayMode', settingsStore.appearance.albumDisplayMode === 'grid' ? 'list' : 'grid');
              }}
            >
              <p class='text-secondary-text px-1 text-center text-xs leading-0 font-bold tracking-tight'>
                {settingsStore.appearance.albumDisplayMode === 'grid' ? 'Grid' : 'List'}
              </p>
              <i
                class={`${settingsStore.appearance.albumDisplayMode === 'grid' ? 'icon-[pixelarticons--grid-2x2-2]' : 'icon-[pixelarticons--bulletlist-sharp]'} text-[13px]`}
              />
            </div>
          </div>
          <Switch>
            <Match when={!artist()}>
              <LoadCircle sizeClass='text-[32px] m-1' class='self-center justify-self-center' />
            </Match>
            <Match when={artist() && settingsStore.appearance.albumDisplayMode === 'grid'}>
              <div class='p-3'>
                <AlbumGrid
                  albums={artist()?.albums ?? []}
                  inline={true}
                  noArtistName={true}
                  emptyMessage='No ID3 albums returned for this artist.'
                  onItemClick={(album) => {
                    navigate(resolveAlbumRoute(album.id));
                  }}
                />
              </div>
            </Match>
            <Match when={artist() && settingsStore.appearance.albumDisplayMode === 'list'}>
              <AlbumList
                albums={artist()?.albums ?? []}
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
    </div>
  );
}

export default BrowseArtist;
