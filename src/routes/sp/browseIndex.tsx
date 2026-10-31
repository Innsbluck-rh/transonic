import { createSignal, Match, Switch } from 'solid-js';
import RouteHeader from '~/components/common/text/RouteHeader';
import IndexModeSelect from '~/components/sidebar/index/IndexModeSelect';
import ArtistList from '~/components/sidebar/index/list/ArtistList';
import FolderList from '~/components/sidebar/index/list/FolderList';
import { type BrowseMode } from '~/features/navigation/routes';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

function SPBrowseIndex() {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Artists');

  return (
    <div class='home-surface-root relative flex size-full flex-col gap-0 overflow-y-auto p-0'>
      <RouteHeader title='Browse' />
      <div class='sticky top-12 z-10 flex h-0 justify-end'>
        <div class='p-1.5'>
          <IndexModeSelect
            defaultMode={browseMode()}
            onSelect={(mode) => {
              setBrowseMode(mode);
            }}
          />
        </div>
      </div>
      <div
        class='flex w-full flex-1 flex-col'
        style={{
          'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
          'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
        }}
      >
        <Switch>
          <Match when={browseMode() === 'Artists'}>
            <ArtistList />
          </Match>
          <Match when={browseMode() === 'Folder Structures'}>
            <FolderList />
          </Match>
        </Switch>
      </div>
    </div>
  );
}

export default SPBrowseIndex;
