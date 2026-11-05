import { Component, JSX, Show } from 'solid-js';
import { clearQueueSelection, isQueueSelecting, queueSelection, removeSelectedQueueRows, selectAllQueueRows } from '~/stores/QueueSelectionStore';

// Same look as the queue header's "Clear".
const ToolbarAction: Component<{ label?: string; onClick: () => void; children: JSX.Element }> = (props) => (
  <div role='button' aria-label={props.label} title={props.label} class='group flex cursor-pointer items-center' onClick={() => props.onClick()}>
    <p class='text-secondary-text group-hover:text-accent flex items-center px-1 text-center text-xs font-bold tracking-tight whitespace-nowrap'>
      {props.children}
    </p>
  </div>
);

/** Shown under the queue list while rows are selected. */
const QueueSelectionToolbar: Component = () => {
  const count = () => queueSelection().indices.length;

  return (
    <Show when={isQueueSelecting()}>
      <div data-queue-selection-scope class='border-primary-border flex shrink-0 flex-row items-center border-t px-2 py-1.5'>
        <p class='archivo mr-auto truncate px-1 text-xs font-bold'>
          {count()} {count() === 1 ? 'item' : 'items'} selected
        </p>
        <ToolbarAction onClick={selectAllQueueRows}>Select All</ToolbarAction>
        <ToolbarAction onClick={() => removeSelectedQueueRows()}>Remove from Queue</ToolbarAction>
        <ToolbarAction label='Deselect' onClick={clearQueueSelection}>
          <i class='icon-[material-symbols--close] text-sm' />
        </ToolbarAction>
      </div>
    </Show>
  );
};

export default QueueSelectionToolbar;
