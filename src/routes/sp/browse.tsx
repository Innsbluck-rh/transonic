import { createSignal, Match, Switch } from 'solid-js';
import RouteHeader from '~/components/common/RouteHeader';
import ArtistList from '~/components/sidebar/index/list/ArtistList';
import FolderList from '~/components/sidebar/index/list/FolderList';
import IndexModeSelect2 from '~/components/sidebar/index/select/IndexModeSelect2';
import { type BrowseMode } from '~/features/navigation/routes';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

function SPBrowseIndex() {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Artists');

  return (
    <div class='home-surface-root relative flex w-full flex-col gap-0 overflow-y-auto p-0'>
      <RouteHeader title='Browse'>
        <div class='flex-1'></div>
        <div class='p-1.5'>
          <IndexModeSelect2
            defaultMode={browseMode()}
            onSelect={(mode) => {
              setBrowseMode(mode);
            }}
          />
        </div>
      </RouteHeader>
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
