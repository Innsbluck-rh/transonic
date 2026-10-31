import interact from 'interactjs';
import { usePlayback } from '~/features/playback/usePlayback';
import { buildQueueAutoScrollKey, useQueueAutoScroll } from '~/features/playback/useQueueAutoScroll';
import QueueList from '../common/list/song/QueueList';
import Heading3 from '../common/text/Heading3';

const RightSideBar = () => {
  interact('.queue-sidebar').resizable({
    edges: { left: true },
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

  return (
    <div class='bg-primary-plane queue-sidebar border-primary-border flex min-h-0 w-80 max-w-[50%] min-w-56 flex-col gap-0 overflow-y-hidden border-l'>
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
        <div ref={setScrollContainerRef} class='flex-1 overflow-y-auto pb-2'>
          <QueueList queue={queue()} itemRef={setItemRef} />
        </div>
      </div>
      {/*<div class='bg-primary-border h-px w-full' />
      <ConnectSidebarSection />*/}
    </div>
  );
};

export default RightSideBar;
