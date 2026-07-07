import { Icon } from '@iconify-icon/solid';
import { Component, createSignal, Show } from 'solid-js';
import { formatCompactDuration } from '~/utils/duration';
import Heading3 from '../text/Heading3';

const [open, setOpen] = createSignal(false);

export function openConnectQueueResolveDialog() {
  setOpen(true);
}
export function closeConnectQueueResolveDialog() {
  setOpen(false);
}

// 実装が完了したらこれを含めコメントは削除すること。
// Connectの有効化時において空でないローカルQueueがあり、かつConnect側のQueueと衝突した場合、
// このダイアログを表示してユーザーにローカルvsConnect側を選択させる。
// 選択されたら、選択された方に応じてPush/Pullを行う。
export default function ConnectQueueResolveDialog() {
  // TODO: Exchangeするキューをどこから取得すべきか決める(open時に渡すか、effectなどでここで取得するか)
  // 現在の見解として、このダイアログはあくまでも「localとconnectのキューをどちらも表示し、選ばれたらpush/pull処理に移行する」というだけのダイアログであり、
  // 表示側のコンテキストにはそこまで左右されないので内側で完結していいと思われる

  return (
    <Show when={open()}>
      <div class='fixed inset-0 z-100 flex items-center justify-center bg-black/45 p-4' role='presentation'>
        <section
          class='border-primary-border bg-primary-plane text-primary-text flex max-h-[80dvh] w-full max-w-2xl flex-col overflow-hidden rounded border shadow-xl'
          role='alertdialog'
          aria-modal='true'
          aria-labelledby='playback-error-title'
        >
          <header class='border-secondary-border flex items-center gap-3 border-b px-4 py-3'>
            <h2 id='playback-error-title' class='text-primary-text min-w-0 flex-1 text-base'>
              Resolve Queue
            </h2>
            <button class='button-borderless px-2' type='button' onClick={close} aria-label='Close playback error'>
              <Icon icon='material-symbols:close' />
            </button>
          </header>
          <div class='flex flex-col gap-1 p-4'>
            <ResolvingQueueItem name='Server' icon='material-symbols:host-sharp' />

            <div class='flex items-center justify-end justify-evenly gap-2 px-4 py-3'>
              <button class='flex items-center gap-1' type='button'>
                <Icon icon='material-symbols:arrow-upward' />
                <span>Push Local</span>
              </button>
              <button class='flex items-center gap-1' type='button'>
                <Icon icon='material-symbols:arrow-downward' />
                <span>Pull From Server</span>
              </button>
            </div>

            <ResolvingQueueItem name='Local' icon='material-symbols:devices-sharp' />
          </div>

          <footer class='border-secondary-border flex items-center justify-end gap-2 border-t px-4 py-3'>
            <button type='button' onClick={closeConnectQueueResolveDialog}>
              Cancel
            </button>
          </footer>
        </section>
      </div>
    </Show>
  );
}

interface ResolvingQueueItemProps {
  name: string;
  icon: string;
  // etc
}

const ResolvingQueueItem: Component<ResolvingQueueItemProps> = (props) => {
  return (
    <div class='flex cursor-pointer flex-col gap-2 rounded-md px-3 py-2'>
      <div class='flex items-center gap-2'>
        <Icon icon={props.icon} />
        <Heading3>{props.name}</Heading3>
        <div class='flex-1' />
        <p class='font-bold'>10 items / playing</p>
      </div>
      <div class='bg-primary-surface ripple relative flex h-auto flex-row items-center overflow-x-hidden rounded-md px-3 py-2'>
        <Show when={true} fallback={<div class='border-secondary-border mr-3 aspect-square max-h-8 w-8 rounded-md border' />}>
          {(assetUrl) => <img src={''} class='mr-3 aspect-square max-h-8 w-8 rounded-md' loading='lazy' decoding='async' />}
        </Show>

        <div class='flex min-w-0 flex-1 flex-col gap-0'>
          <div class='flex items-center gap-1'>
            <p class='song-item-title archivo gap-1 truncate font-bold'>title</p>
          </div>
          <p class='song-item-meta archivo truncate text-[11px]'>artist</p>
        </div>

        <p class='song-item-meta archivo ml-2 text-xs'>{formatCompactDuration(210)}</p>
      </div>
    </div>
  );
};
