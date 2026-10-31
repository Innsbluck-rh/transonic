import { createSignal } from 'solid-js';
import RouteHeader from '~/components/common/text/RouteHeader';
import BrowseItemList from '~/components/sidebar/browse/BrowseItemList';
import BrowseModeSelect from '~/components/sidebar/browse/BrowseModeSelect';
import { type BrowseMode } from '~/features/navigation/routes';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

function SPBrowseRoot() {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Artists');

  return (
    <div class='home-surface-root relative flex size-full flex-col gap-0 overflow-y-auto p-0'>
      <RouteHeader title='Browse' />
      <div class='sticky top-12 z-10 flex h-0 justify-end'>
        <div class='p-1.5'>
          <BrowseModeSelect
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
        <BrowseItemList mode={browseMode()} />
      </div>
    </div>
  );
}

export default SPBrowseRoot;
