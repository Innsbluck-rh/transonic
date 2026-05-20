import { useLocation } from '@solidjs/router';
import { Component, For, Match, Show, Switch } from 'solid-js';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import BrowseListRow from './item/BrowseListRow';

export type BrowseListItem = {
  id: string;
  name: string;
  href?: string;
};

interface BrowseListProps {
  items: BrowseListItem[];
  isLoading?: boolean; // this should be removed
  error?: string | null; // this should be removed
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
      <Switch>
        <Match when={props.isLoading}>
          <p class='px-3 py-2 text-xs text-zinc-500'>Loading browse items...</p>
        </Match>

        <Match when={props.error}>{(message) => <p class='px-3 py-2 text-xs text-red-500'>{message()}</p>}</Match>

        <Match when={props.items.length > 0}>
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
        </Match>

        <Match when={true}>
          <Show when={props.emptyMessage}>
            <p class='px-3 py-2 text-xs text-zinc-500'>{props.emptyMessage}</p>
          </Show>
        </Match>
      </Switch>
    </div>
  );
};

export default BrowseList;
