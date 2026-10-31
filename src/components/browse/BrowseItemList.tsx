import { Component, createResource, Show } from 'solid-js';
import LoadCircle from '~/components/common/LoadCircle';
import BrowseList from '~/components/common/list/browse/BrowseList';
import { loadBrowseItems } from '~/features/browse/service';
import { type BrowseMode } from '~/features/navigation/routes';

interface BrowseItemListProps {
  mode: BrowseMode;
}

const BrowseItemList: Component<BrowseItemListProps> = (props) => {
  const [items] = createResource(() => props.mode, loadBrowseItems);

  return (
    <Show
      when={!items.loading}
      fallback={
        <div class='flex size-full items-center justify-center'>
          <LoadCircle />
        </div>
      }
    >
      <BrowseList items={items() ?? []} emptyMessage='Nothing Found in Library' />
    </Show>
  );
};

export default BrowseItemList;
