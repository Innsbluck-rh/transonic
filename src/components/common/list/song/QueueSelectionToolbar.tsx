import { Component, JSX, Show } from 'solid-js';
import { clearQueueSelection, isQueueSelecting, queueSelection, removeSelectedQueueRows, selectAllQueueRows } from '~/stores/QueueSelectionStore';

const ToolbarAction: Component<{ label?: string; onClick: () => void; children: JSX.Element }> = (props) => (
  <div role='button' aria-label={props.label} title={props.label} class='group flex cursor-pointer items-center' onClick={() => props.onClick()}>
    <p class='group-hover:text-accent ripple -m-1.5 flex items-center overflow-visible rounded-full p-2'>{props.children}</p>
  </div>
);

/**
 * Floats over the bottom of the queue list while rows are selected. It sits outside the
 * list's scroller, so Android's overscroll stretch leaves it alone; a `placeholder` copy at
 * the end of the scroller keeps the last rows clear of it.
 */
const QueueSelectionToolbar: Component<{ placeholder?: boolean }> = (props) => {
  const count = () => queueSelection().indices.length;

  return (
    <Show when={isQueueSelecting()}>
      <div
        aria-hidden={props.placeholder ? 'true' : undefined}
        class='bg-primary-surface border-secondary-border mx-1 mt-2 flex flex-row items-center gap-2.5 rounded-md border px-2.5 py-2 shadow-md'
        classList={{ invisible: props.placeholder, 'absolute inset-x-0 bottom-2 z-10': !props.placeholder }}
      >
        <p class='mr-auto ml-1 truncate font-bold'>
          {count()} {count() === 1 ? 'item' : 'items'} selected
        </p>
        <ToolbarAction label='Remove from Queue' onClick={() => removeSelectedQueueRows()}>
          <i class='icon-[material-symbols--playlist-remove] text-xl' />
        </ToolbarAction>
        <ToolbarAction label='Select All' onClick={selectAllQueueRows}>
          <i class='icon-[material-symbols--select-all] text-xl' />
        </ToolbarAction>
        <ToolbarAction label='Deselect' onClick={clearQueueSelection}>
          <i class='icon-[material-symbols--close] text-xl' />
        </ToolbarAction>
      </div>
    </Show>
  );
};

export default QueueSelectionToolbar;
