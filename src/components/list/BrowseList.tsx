import { Component, For, Match, Show, Switch } from 'solid-js';
import BrowseListRow from './item/browse/BrowseListRow';

type BrowseListItem = {
  id: string;
  name: string;
};

interface BrowseListProps {
  items: BrowseListItem[];
  isLoading: boolean;
  error: string | null;
  emptyMessage: string;
  onClickItem?: (item: BrowseListItem) => void;
}

const BrowseList: Component<BrowseListProps> = (props) => {
  return (
    <div class='flex flex-col'>
      <Switch>
        <Match when={props.isLoading}>
          <p class='px-3 py-2 text-xs text-zinc-500'>Loading browse items...</p>
        </Match>

        <Match when={props.error}>{(message) => <p class='px-3 py-2 text-xs text-red-500'>{message()}</p>}</Match>

        <Match when={props.items.length > 0}>
          <For each={props.items}>{(item) => <BrowseListRow label={item.name} onClick={() => props.onClickItem?.(item)} />}</For>
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
