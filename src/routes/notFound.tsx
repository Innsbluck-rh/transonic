import { Icon } from '@iconify-icon/solid';
import { useLocation, useNavigate } from '@solidjs/router';
import Heading1 from '~/components/common/Heading1';

function NotFound() {
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <div class='flex flex-col gap-4 items-center justify-center h-dvh w-dvw bg-zinc-100'>
      <div class='flex flex-col gap-4 p-4  bg-zinc-100'>
        <Icon icon='pixelarticons:fire' class='text-8xl w-fit' />
        <Heading1>Oops!</Heading1>
        <p class='archivo italic opacity-75'>This Page ({location.pathname}) is not available.</p>
        <button class='w-fit' onClick={() => navigate(-1)}>
          Back
        </button>
      </div>
    </div>
  );
}

export default NotFound;
