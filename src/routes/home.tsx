import { createMemo, For, Match, Show, Switch } from 'solid-js';
import AlbumGrid, { getAlbumGridColumnCount } from '~/components/common/list/album/AlbumGrid';
import HorizontalAlbumItem from '~/components/common/list/album/item/HorizontalAlbumItem';
import LoadCircle from '~/components/common/LoadCircle';
import Heading2 from '~/components/common/text/Heading2';
import RouteHeader from '~/components/common/text/RouteHeader';
import { useAlbumGridWidth } from '~/features/home/useAlbumGridWidth';
import { useHomeAlbumList } from '~/features/home/useHomeAlbumList';
import { resolveAlbumRoute } from '~/features/navigation/routes';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';

function Home() {
  const navigate = useSPNavigate();
  const { albumGridWidth: raAlbumGridWidth, observeRef: raObserveRef } = useAlbumGridWidth();
  const { albumGridWidth: rndAlbumGridWidth, observeRef: rndObserveRef } = useAlbumGridWidth();

  // count that enough to fill one row in 4k displays (to fetch enough amount of albums)
  const gridMaxFetchCount = getAlbumGridColumnCount(3840);

  const { albums: rpAlbums, initLoading: rpInitLoading, error: rpError } = useHomeAlbumList('recent', 6);
  const { albums: raAlbums, initLoading: raInitLoading, error: raError } = useHomeAlbumList('newest', gridMaxFetchCount);
  const { albums: rndAlbums, initLoading: rndInitLoading, error: rndError, refetch: rndRefetch } = useHomeAlbumList('random', gridMaxFetchCount);

  const isInitLoadingAnyAlbums = createMemo(() => rpInitLoading() || raInitLoading() || rndInitLoading());

  return (
    <div class='home-surface-root overflow-y-auto p-0'>
      <RouteHeader title='Home' />
      <Show
        when={!isInitLoadingAnyAlbums()}
        fallback={
          <div class='flex size-full items-center justify-center'>
            <LoadCircle />
          </div>
        }
      >
        <div class='mt-2 flex w-full flex-col gap-3 pb-4'>
          <div class='flex w-full flex-col overflow-x-hidden'>
            <Heading2 class='px-2'>recently played</Heading2>
            <Switch>
              <Match when={rpError()}>{(error) => <p class='archivo text-secondary-text my-3'>Error: {error()}</p>}</Match>
              <Match when={rpAlbums()?.length === 0}>
                <p class='archivo text-secondary-text my-3 text-center italic'>No recent albums.</p>
              </Match>
              <Match when={rpAlbums()}>
                {(albums) => (
                  <div class='grid w-full gap-x-3 gap-y-2.5 p-3 sm:grid-cols-1 md:grid-cols-2 xl:grid-cols-3'>
                    <For each={albums()}>
                      {(album, _i) => {
                        return <HorizontalAlbumItem album={album} />;
                      }}
                    </For>
                  </div>
                )}
              </Match>
            </Switch>
          </div>

          <div class='flex w-full flex-col overflow-x-hidden'>
            <Heading2 class='px-2'>recently added</Heading2>
            <Switch>
              <Match when={raError()}>{(error) => <p class='archivo text-secondary-text my-3'>Error: {error()}</p>}</Match>
              <Match when={raAlbums()}>
                {(albums) => (
                  <div class='m-3' ref={raObserveRef}>
                    <AlbumGrid
                      inline
                      albums={albums().slice(0, getAlbumGridColumnCount(raAlbumGridWidth()))}
                      emptyMessage='No recently added albums.'
                      onItemClick={(album) => {
                        navigate(resolveAlbumRoute(album.id));
                      }}
                    />
                  </div>
                )}
              </Match>
            </Switch>
          </div>

          <div class='flex w-full flex-col overflow-x-hidden'>
            <div class='flex justify-between px-2'>
              <Heading2>random picks</Heading2>
              <div class='ripple mr-2 cursor-pointer rounded-full p-1' onClick={() => rndRefetch()}>
                <i class='icon-[material-symbols--refresh] text-xl' />
              </div>
            </div>
            <Switch>
              <Match when={rndError()}>{(error) => <p class='archivo text-secondary-text my-3'>Error: {error()}</p>}</Match>
              <Match when={rndAlbums()}>
                {(albums) => (
                  <div class='m-3' ref={rndObserveRef}>
                    <AlbumGrid
                      inline
                      albums={albums().slice(0, getAlbumGridColumnCount(rndAlbumGridWidth()))}
                      emptyMessage='No albums.'
                      onItemClick={(album) => {
                        navigate(resolveAlbumRoute(album.id));
                      }}
                    />
                  </div>
                )}
              </Match>
            </Switch>
          </div>
        </div>
      </Show>
    </div>
  );
}

export default Home;
