import { createSignal, Match, Switch } from 'solid-js';
import Heading1 from '~/components/common/Heading1';
import { BrowseMode } from '~/components/sidebar/index/IndexContent';
import ArtistList from '~/components/sidebar/index/list/ArtistList';
import FolderList from '~/components/sidebar/index/list/FolderList';
import IndexModeSelect2 from '~/components/sidebar/index/select/IndexModeSelect2';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

function SPBrowseIndex() {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Artists');

  return (
    <div class='home-surface-root flex w-full flex-col gap-0 overflow-y-hidden p-0'>
      <div class='border-primary-border bg-primary-bg-primary-surface flex w-full items-end border-b px-2 pt-3 pb-2'>
        <Heading1>Browse</Heading1>
        <div class='flex-1'></div>
        <IndexModeSelect2
          defaultMode={browseMode()}
          onSelect={(mode) => {
            setBrowseMode(mode);
          }}
        />
      </div>

      <div
        class='flex h-full w-full flex-col overflow-y-auto'
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
