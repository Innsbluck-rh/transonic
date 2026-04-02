import { Icon } from '@iconify-icon/solid';
import { Component, For, Show } from 'solid-js';
import { usePlayback } from '~/features/playback/usePlayback';

const QueueContent: Component = () => {
  const { queue, isQueueIndexActive, playQueueIndex } = usePlayback();

  return (
    <div class='flex min-h-0 flex-1 overflow-x-hidden overflow-y-auto'>
      <div class='grid h-fit w-full grid-cols-[max-content_minmax(0,1fr)]'>
        <For
          each={queue()}
          fallback={
            <div class='col-span-2 flex w-full p-2'>
              <p class='text-secondary-text px-1 text-xs italic'>[nothing in queue]</p>
            </div>
          }
        >
          {(queueEntry, i) => {
            const onClickEntry = async () => {
              await playQueueIndex(i());
            };

            const isPlaying = isQueueIndexActive(i());

            return (
              <div
                class='hover:bg-primary-hover col-span-2 grid w-full cursor-pointer grid-cols-subgrid items-center gap-x-3 px-3 py-2'
                classList={{
                  'bg-primary-playing': isPlaying,
                }}
                onClick={onClickEntry}
              >
                <div class='flex flex-row items-start'>
                  <Show when={!isPlaying} fallback={<Icon class='text-accent -ml-0.5 w-fit scale-125 text-xs' icon='material-symbols:play-arrow' />}>
                    <p class='archivo text-xs font-bold'>{i() + 1}</p>
                  </Show>
                </div>

                <p
                  class='min-w-0 truncate text-xs'
                  classList={{
                    'text-accent font-bold': isPlaying,
                  }}
                  title={queueEntry.title}
                >
                  {queueEntry.title}
                </p>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};

export default QueueContent;
