import { Component, createEffect, createSignal, For } from 'solid-js';
import { PlaybackQueueEntry } from '~/bindings';
import Heading3 from '~/components/common/Heading3';
import { playbackStore } from '~/stores/PlaybackStore';

const QueueSideBar: Component = () => {
  const [queue, setQueue] = createSignal<PlaybackQueueEntry[] | undefined>(playbackStore.status?.queue);

  createEffect(() => {
    setQueue(playbackStore.status?.queue);
  });

  return (
    <div class='flex flex-col min-w-40 w-56 bg-primary-plane border-l border-primary-border resize-x overflow-auto'>
      <Heading3 class='m-2 w-fit'>Queue</Heading3>
      <div class='flex flex-col w-full overflow-x-hidden overflow-y-auto '>
        <For
          each={queue()}
          fallback={
            <div class='flex flex-row p-2 w-full'>
              <p class='text-xs italic'>[nothing in queue]</p>
            </div>
          }
        >
          {(queueEntry, i) => {
            return (
              <div class='flex flex-row px-3 py-2 w-full items-center gap-2 cursor-pointer hover:bg-primary-surface'>
                <p class='text-xs archivo font-bold'>{i() + 1}</p>
                <p class='text-xs flex-1 text-ellipsis'>{queueEntry.songId}</p>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};

export default QueueSideBar;
