import { Component } from 'solid-js';
import QueueList from '~/components/common/list/song/QueueList';
import Heading3 from '~/components/common/text/Heading3';
import { usePlayback } from '~/features/playback/usePlayback';
import { buildQueueAutoScrollKey, useQueueAutoScroll } from '~/features/playback/useQueueAutoScroll';

const QueueSidebarSection: Component = () => {
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
  );
};

export default QueueSidebarSection;
