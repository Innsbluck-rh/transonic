import { useLocation } from '@solidjs/router';
import { Component, For, Show } from 'solid-js';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import BrowseListRow from './item/BrowseListRow';

export type BrowseListItem = {
  id: string;
  name: string;
  href?: string;
};

interface BrowseListProps {
  items: BrowseListItem[];
  emptyMessage: string;
  selectedId?: string | null;
  onClickItem?: (item: BrowseListItem) => void;
}

/**
 * @description Indexなどのための汎用リスト
 */
const BrowseList: Component<BrowseListProps> = (props) => {
  const location = useLocation();
  const navigate = useSPNavigate();
  return (
    <div class='flex flex-col pb-3'>
      <Show
        when={props.items.length > 0}
        fallback={
          <Show when={props.emptyMessage}>
            <p class='px-3 py-2 text-xs text-zinc-500'>{props.emptyMessage}</p>
          </Show>
        }
      >
        <For each={props.items}>
          {(item) => (
            <>
              <BrowseListRow
                label={item.name}
                active={item.href ? location.pathname.startsWith(item.href) : false}
                selected={props.selectedId === item.id}
                onClick={() => {
                  props.onClickItem?.(item);
                  if (item.href) navigate(item.href);
                }}
              />
              <div class='bg-secondary-border h-px w-full'></div>
            </>
          )}
        </For>
      </Show>
    </div>
  );
};

export default BrowseList;
