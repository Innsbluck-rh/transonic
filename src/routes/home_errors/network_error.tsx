import { Icon } from '@iconify-icon/solid';
import { reload } from '@solidjs/router';
import Heading1 from '~/components/common/Heading1';

function HomeNetworkError() {
  return (
    <div class='flex flex-col gap-4 items-center justify-center h-dvh w-dvw bg-zinc-100'>
      <div class='flex flex-col gap-4 p-4  bg-zinc-100'>
        <Icon icon='pixelarticons:fire' class='text-8xl w-fit' />
        <Heading1>Oops!</Heading1>
        <p class='archivo italic opacity-75'>No internet connection.</p>
        <button class='w-fit' onClick={() => reload()}>
          Refresh
        </button>
      </div>
    </div>
  );
}

export default HomeNetworkError;
