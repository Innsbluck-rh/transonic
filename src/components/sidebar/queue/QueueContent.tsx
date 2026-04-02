import { Icon } from '@iconify-icon/solid';
import { Component, createEffect, createSignal, For, Show } from 'solid-js';
import { commands, PlaybackQueueEntry } from '~/bindings';
import { hasPlaybackCommandError } from '~/features/playback/service';
import { playbackStore } from '~/stores/PlaybackStore';

const QueueContent: Component = () => {
  const [queue, setQueue] = createSignal<PlaybackQueueEntry[] | undefined>(playbackStore.status?.queue);

  createEffect(() => {
    setQueue(playbackStore.status?.queue);
  });

  return (
    <div class='flex flex-1 min-h-0 overflow-x-hidden overflow-y-auto'>
      <div class='grid h-fit w-full grid-cols-[max-content_minmax(0,1fr)]'>
        <For
          each={queue()}
          fallback={
            <div class='col-span-2 flex w-full p-2'>
              <p class='px-1 text-xs text-secondary-text italic'>[nothing in queue]</p>
            </div>
          }
        >
          {(queueEntry, i) => {
            const onClickEntry = async () => {
              const entries = queue();
              const jumpIndex = i();
              if (!entries) return;
              const sqResult = await commands.playbackSetQueue({
                currentIndex: jumpIndex,
                entries,
              });
              if (hasPlaybackCommandError(sqResult)) {
                return;
              }
              const playResult = await commands.playbackPlay();
              hasPlaybackCommandError(playResult);
            };

            const isPlaying = playbackStore.status?.currentIndex === i();

            return (
              <div
                class='col-span-2 grid w-full cursor-pointer grid-cols-subgrid items-center gap-x-3 px-3 py-2 hover:bg-primary-hover'
                classList={{
                  'bg-primary-playing': isPlaying,
                }}
                onClick={onClickEntry}
              >
                <div class='flex flex-row items-start'>
                  <Show when={!isPlaying} fallback={<Icon class='-ml-0.5 text-xs w-fit scale-125 text-accent' icon='material-symbols:play-arrow' />}>
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
