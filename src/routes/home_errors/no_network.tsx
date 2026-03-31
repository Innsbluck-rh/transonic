import { Icon } from '@iconify-icon/solid';
import Heading1 from '~/components/common/Heading1';

function HomeNoNetwork() {
  return (
    <div class='flex flex-col gap-4 items-center justify-center h-full w-full bg-zinc-100'>
      <div class='flex flex-col gap-4 p-4 min-w-[40%]  bg-zinc-100'>
        <Icon icon='pixelarticons:fire' class='text-8xl w-fit text-zinc-700' />
        <Heading1>Oops!</Heading1>
        <p class='archivo italic opacity-75'>No internet connection.</p>
        <button class='w-fit' onClick={() => window.location.reload()}>
          Refresh
        </button>
      </div>
    </div>
  );
}

export default HomeNoNetwork;
