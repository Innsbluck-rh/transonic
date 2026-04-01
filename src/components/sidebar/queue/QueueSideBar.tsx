import { Component, createEffect, createResource, createSignal, For, Suspense } from 'solid-js';
import { commands, PlaybackQueueEntry } from '~/bindings';
import Heading3 from '~/components/common/Heading3';
import { hasPlaybackCommandError } from '~/features/playback/service';
import { playbackStore } from '~/stores/PlaybackStore';

const QueueSideBar: Component = () => {
  const [queue, setQueue] = createSignal<PlaybackQueueEntry[] | undefined>(playbackStore.status?.queue);

  createEffect(() => {
    setQueue(playbackStore.status?.queue);
  });

  return (
    <div class='flex flex-col min-w-40 w-56 bg-primary-plane border-l border-primary-border resize-x overflow-auto'>
      <Heading3 class='m-2 w-fit'>Queue</Heading3>
      <div class='grid w-full grid-cols-[max-content_minmax(0,1fr)] overflow-x-hidden overflow-y-auto'>
        <For
          each={queue()}
          fallback={
            <div class='col-span-2 flex w-full p-2'>
              <p class='text-xs italic'>[nothing in queue]</p>
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

            const [songName] = createResource(queueEntry.songId, async () => {
              const songResult = await commands.getSong({ id: queueEntry.songId });
              if (songResult.status === 'error') {
                return '[Error]';
              } else {
                return songResult.data.title;
              }
            });

            return (
              <div
                class='col-span-2 grid w-full cursor-pointer grid-cols-subgrid items-center gap-x-2 px-3 py-2 hover:bg-primary-surface'
                onClick={onClickEntry}
              >
                <p class='text-xs archivo font-bold'>{i() + 1}</p>
                <Suspense fallback={<p class='min-w-0 truncate text-xs italic'>Loading...</p>}>
                  <p class='min-w-0 truncate text-xs' title={songName() ?? undefined}>
                    {songName()}
                  </p>
                </Suspense>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
};

export default QueueSideBar;
