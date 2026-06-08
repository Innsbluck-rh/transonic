import { Icon } from '@iconify-icon/solid';
import interact from 'interactjs';
import { Component } from 'solid-js';
import Heading3 from '~/components/common/Heading3';
import QueueList from '~/components/common/list/song/QueueList';
import { usePlayback } from '~/features/playback/usePlayback';
import { buildQueueAutoScrollKey, useQueueAutoScroll } from '~/features/playback/useQueueAutoScroll';

const QueueSideBar: Component = () => {
  const { queue, currentEntry, currentIndex, clearQueue } = usePlayback();
  const { setScrollContainerRef, setItemRef } = useQueueAutoScroll({
    currentIndex,
    currentEntryId: () => currentEntry()?.id ?? null,
    queueKey: () => buildQueueAutoScrollKey(queue()),
    defaultBehavior: 'smooth',
    initialBehavior: 'instant',
    queueChangeBehavior: 'instant',
  });

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
  return (
    <div class='bg-primary-plane queue-sidebar border-primary-border flex h-full w-80 max-w-[50%] min-w-56 flex-col overflow-y-hidden border-l'>
      <div class='border-secondary-border flex w-full flex-row items-center border-b px-2 py-1'>
        <Heading3>Queue</Heading3>
        <div class='flex-1'></div>
        <Icon
          icon='pixelarticons:loader'
          class='cursor-pointer text-[13px]'
          title='Clear Queue'
          onClick={() => {
            clearQueue();
          }}
        />
      </div>
      <div ref={setScrollContainerRef} class='flex-1 overflow-y-auto'>
        <QueueList queue={queue()} itemRef={setItemRef} />
      </div>
    </div>
  );
};

export default QueueSideBar;
