import interact from 'interactjs';
import { createSignal } from 'solid-js';
import { usePlayback } from '~/features/playback/usePlayback';
import { buildQueueAutoScrollKey, useQueueAutoScroll } from '~/features/playback/useQueueAutoScroll';
import QueueList from '../common/list/song/QueueList';
import QueueSelectionToolbar from '../common/list/song/QueueSelectionToolbar';
import Heading3 from '../common/text/Heading3';

const RightSideBar = () => {
  interact('.queue-sidebar').resizable({
    // A handle element rather than `left: true`, so a press on the edge resizes
    // without also reaching the queue rows underneath (which drag on press).
    edges: { left: '.queue-sidebar-resize-edge' },
    listeners: {
      move: function (event) {
        event.preventDefault();
        event.stopImmediatePropagation();
        event.stopPropagation();

        let { x, y } = event.target.dataset;

        x = (parseFloat(x) || 0) + event.deltaRect.left;
        y = (parseFloat(y) || 0) + event.deltaRect.top;

        // 要素のサイズと位置を更新
        Object.assign(event.target.style, {
          width: `${event.rect.width}px`,
          height: `${event.rect.height}px`,
        });

        Object.assign(event.target.dataset, { x, y });
      },
    },
  });

  const { queue, currentEntry, currentIndex, clearQueue } = usePlayback();
  const { setScrollContainerRef, setItemRef } = useQueueAutoScroll({
    currentIndex,
    currentEntryId: () => currentEntry()?.id ?? null,
    queueKey: () => buildQueueAutoScrollKey(queue()),
    defaultBehavior: 'smooth',
    initialBehavior: 'instant',
    queueChangeBehavior: 'instant',
  });
  const [scrollContainer, setScrollContainer] = createSignal<HTMLDivElement>();

  return (
    <div class='bg-primary-plane queue-sidebar border-primary-border relative flex min-h-0 w-80 max-w-[50%] min-w-56 flex-col gap-0 overflow-y-hidden border-l'>
      <div class='flex min-h-0 w-full flex-1 flex-col overflow-y-auto'>
        <div class='border-primary-border flex w-full flex-row items-center border-b px-2 py-1'>
          <Heading3 class='text-secondary-text'>Queue</Heading3>
          <div class='flex-1'></div>
          <div
            class='group flex cursor-pointer items-center'
            onClick={() => {
              clearQueue();
            }}
          >
            <p class='text-secondary-text group-hover:text-accent px-1 text-center text-xs font-bold tracking-tight'>Clear</p>
          </div>
        </div>
        <div class='relative flex min-h-0 flex-1 flex-col'>
          <div
            ref={(element) => {
              setScrollContainerRef(element);
              setScrollContainer(element);
            }}
            class='flex flex-1 flex-col overflow-y-auto pb-2'
          >
            <QueueList queue={queue()} scrollContainer={scrollContainer()} itemRef={setItemRef} />
            <QueueSelectionToolbar placeholder />
          </div>
          <QueueSelectionToolbar />
        </div>
      </div>
      {/*<div class='bg-primary-border h-px w-full' />
      <ConnectSidebarSection />*/}
      {/* Same 20px as interact's default resize margin the edge used before. */}
      <div class='queue-sidebar-resize-edge absolute inset-y-0 left-0 z-30 w-5' />
    </div>
  );
};

export default RightSideBar;
