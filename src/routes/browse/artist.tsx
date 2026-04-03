import { useParams, type RouteSectionProps } from '@solidjs/router';
import { createEffect, createMemo, createResource, createSignal, Show } from 'solid-js';
import { commands, type AlbumListItem, type ArtistInfo2Response, type ArtistResponse } from '~/bindings';
import Heading1 from '~/components/common/Heading1';
import Heading2 from '~/components/common/Heading2';
import LoadCircle from '~/components/common/LoadCircle';
import AlbumGrid from '~/components/common/list/album/AlbumGrid';
import { type BrowseListItem } from '~/components/common/list/index/BrowseList';
import { fetchCoverArtAssetUrl } from '~/features/albums/service';
import { resolveArtistRoute, type RouteVariant } from '~/features/navigation/routes';
import { usePlayback } from '~/features/playback/usePlayback';
import { sessionStore } from '~/stores/SessionStore';

const ARTIST_COVER_ART_SIZE = 320;
const SIMILAR_ARTIST_COUNT = 8;

interface BrowseArtistProps extends Partial<RouteSectionProps<unknown>> {
  routeVariant?: RouteVariant;
}

function BrowseArtist(props: BrowseArtistProps) {
  const params = useParams();
  const { playAlbum } = usePlayback();

  const [artist, setArtist] = createSignal<ArtistResponse | null>(null);
  const [artistInfo, setArtistInfo] = createSignal<ArtistInfo2Response | null>(null);
  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  let latestLoadId = 0;

  const albums = createMemo<AlbumListItem[]>(() =>
    (artist()?.albums ?? []).map((album) => ({
      id: album.id,
      name: album.title ?? album.album ?? album.name ?? '[Untitled album]',
      artist: album.artist ?? artist()?.name ?? null,
      coverArtId: album.coverArtId,
      year: album.year,
    }))
  );

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

  const heroImageUrl = createMemo(
    () => artistInfo()?.largeImageUrl ?? artistInfo()?.mediumImageUrl ?? artistInfo()?.smallImageUrl ?? artist()?.artistImageUrl ?? null
  );

  const coverArtRequest = createMemo(() => {
    if (heroImageUrl()) {
      return null;
    }

    const coverArtId = artist()?.coverArtId;
    const profileId = sessionStore.activeSession?.profileId;
    if (!coverArtId || !profileId) {
      return null;
    }

    return {
      profileId,
      coverArtId,
      size: ARTIST_COVER_ART_SIZE,
    };
  });

  const [coverArt] = createResource(coverArtRequest, (payload) => fetchCoverArtAssetUrl(payload));
  const heroImageSrc = createMemo(() => heroImageUrl() ?? coverArt() ?? null);

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
    <div class='home-surface-root p-3'>
      <Show when={error()}>{(message) => <p class='px-1 text-sm text-red-500'>{message()}</p>}</Show>

      <Show when={!isLoading()} fallback={<LoadCircle class='self-center justify-self-center' />}>
        <Show when={artist()}>
          {(artistValue) => (
            <div class='flex flex-col gap-5'>
              <div class='flex flex-row items-center gap-3'>
                {/* <Show
                  when={heroImageSrc()}
                  fallback={
                    <div class='border-secondary-border bg-primary-selected flex aspect-square w-full max-w-12 items-center justify-center rounded-lg border text-xs text-zinc-500'>
                      No art
                    </div>
                  }
                >
                  {(src) => (
                    <img class='aspect-square h-full max-w-12 rounded-lg object-cover object-center' src={src()} loading='lazy' decoding='async' />
                  )}
                </Show> */}

                <div class='flex min-w-0 flex-1 flex-col gap-1'>
                  <Heading1>{artistValue().name}</Heading1>

                  <Show when={artistInfo()?.lastFmUrl}>
                    {(lastFmUrl) => (
                      <a class='text-accent text-xs underline-offset-2 hover:underline' href={lastFmUrl()} target='_blank' rel='noreferrer'>
                        last.fm
                      </a>
                    )}
                  </Show>
                </div>
              </div>

              {/* <Show when={similarArtists().length > 0}>
                <div class='border-secondary-border bg-primary-plane/50 rounded-xl border py-2'>
                  <Heading2 class='px-4 py-2'>similar artists</Heading2>
                  <BrowseList items={similarArtists()} emptyMessage='' />
                </div>
              </Show> */}

              <div class='flex flex-col gap-2'>
                <Heading2>albums</Heading2>
                <AlbumGrid
                  albums={albums()}
                  emptyMessage='No ID3 albums returned for this artist.'
                  onItemClick={async (album) => {
                    await playAlbum(album.id);
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
