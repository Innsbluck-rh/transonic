import { Component, createMemo, JSX } from 'solid-js';
import { clearQueueSelection, isQueueSelecting, queueSelection, removeSelectedQueueRows, selectAllQueueRows } from '~/stores/QueueSelectionStore';

const ToolbarAction: Component<{ label?: string; onClick: () => void; children: JSX.Element }> = (props) => (
  <div role='button' aria-label={props.label} title={props.label} class='group flex cursor-pointer items-center' onClick={() => props.onClick()}>
    <p class='group-hover:text-accent ripple -m-1.5 flex items-center overflow-visible rounded-full p-2'>{props.children}</p>
  </div>
);

const ToolbarBody: Component<{ count: number; class?: string; hidden?: boolean }> = (props) => (
  <div
    aria-hidden={props.hidden ? 'true' : undefined}
    class={`bg-primary-surface border-secondary-border mx-1 flex flex-row items-center gap-2.5 rounded-md border px-2.5 py-2 shadow-md ${props.class ?? ''}`}
  >
    <p class='mr-auto ml-1 truncate font-bold'>
      {props.count} {props.count === 1 ? 'item' : 'items'} selected
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
);

/**
 * Floats over the bottom of the queue list while rows are selected. It sits outside the
 * list's scroller, so Android's overscroll stretch leaves it alone; a `placeholder` copy at
 * the end of the scroller keeps the last rows clear of it.
 *
 * Both stay mounted and animate in and out together: the toolbar slides up from the bottom
 * edge of the list while the placeholder grows to its height, and both reverse when the
 * selection ends, so the list never jumps by the toolbar's height.
 */
const QueueSelectionToolbar: Component<{ placeholder?: boolean }> = (props) => {
  // Keeps the last count on screen while the toolbar slides out of an emptied selection.
  const count = createMemo<number>((previous) => queueSelection().indices.length || previous, 0);

  if (props.placeholder) {
    return (
      <div
        aria-hidden='true'
        class='grid transition-[grid-template-rows] duration-200 ease-out motion-reduce:transition-none'
        classList={{ 'grid-rows-[1fr]': isQueueSelecting(), 'grid-rows-[0fr]': !isQueueSelecting() }}
      >
        <div class='min-h-0 overflow-hidden'>
          <ToolbarBody count={count()} class='invisible mt-2' />
        </div>
      </div>
    );
  }

  return (
    // Clips the toolbar at the list's bottom edge, so it emerges from there instead of
    // sliding over whatever sits below the list.
    <div class='pointer-events-none absolute inset-x-0 bottom-0 z-10 overflow-y-clip pt-1 pb-2'>
      <ToolbarBody
        count={count()}
        hidden={!isQueueSelecting()}
        class={`pointer-events-auto transition-[translate,visibility] duration-200 ease-out motion-reduce:transition-none ${
          isQueueSelecting() ? '' : 'invisible translate-y-[calc(100%+0.5rem)]'
        }`}
      />
    </div>
  );
};

export default QueueSelectionToolbar;
