import { createSignal } from 'solid-js';
import BrowseItemList from '~/components/browse/BrowseItemList';
import BrowseModeSelect from '~/components/browse/BrowseModeSelect';
import RouteHeader from '~/components/common/text/RouteHeader';
import { type BrowseMode } from '~/features/navigation/routes';
import { SPScrollPaddingStore } from '~/stores/SPScrollPaddingStore';

function SPBrowseIndex() {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Artists');

  return (
    <div class='home-surface-root relative flex size-full flex-col gap-0 overflow-y-auto p-0'>
      <RouteHeader title='Browse' />

      <div class='relative flex size-full flex-col overflow-y-auto'>
        <div class='sticky top-0 z-10 flex items-center px-2 pt-3 pb-2'>
          <div>
            <BrowseModeSelect
              defaultMode={browseMode()}
              onSelect={(mode) => {
                setBrowseMode(mode);
              }}
            />
          </div>
        </div>
        <div
          class='flex size-full flex-col'
          style={{
            'padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
            'scroll-padding-bottom': `${SPScrollPaddingStore.collapsedPlayerHeight}px`,
          }}
        >
          <BrowseItemList mode={browseMode()} />
        </div>
      </div>
    </div>
  );
}

export default SPBrowseIndex;
