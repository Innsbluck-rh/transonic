import { Icon } from '@iconify-icon/solid';
import { useLocation, useNavigate, useSearchParams } from '@solidjs/router';
import { createMemo } from 'solid-js';
import Heading1 from '~/components/common/Heading1';
import { HOME_ERROR_ROUTE, type HomeErrorKind } from '~/features/session/bootstrap';

type AppErrorModel = {
  message: string;
  actionLabel: string;
  action: () => void;
};

function decodePath(pathname: string) {
  try {
    return decodeURIComponent(pathname);
  } catch (_error) {
    return pathname;
  }
}

function readHomeErrorMessage(kind: string | string[] | undefined) {
  const normalizedKind = Array.isArray(kind) ? kind[0] : kind;
  const homeErrorKind = (normalizedKind ?? '') as HomeErrorKind;
  switch (homeErrorKind) {
    case 'no_network':
      return 'No internet connection.';
    case 'connection_error':
      return 'Failed to connect server.';
    case 'reauth_required':
      return 'Reauthentication is required for the active profile.';
    default:
      return 'Failed to restore a previous session.';
  }
}

function AppError() {
  const location = useLocation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  const isHomeErrorRoute = createMemo(() => location.pathname === HOME_ERROR_ROUTE);
  const model = createMemo<AppErrorModel>(() => {
    if (isHomeErrorRoute()) {
      return {
        message: readHomeErrorMessage(searchParams.kind),
        actionLabel: 'Refresh',
        action: () => window.location.reload(),
      };
    }

    return {
      message: `This Page (${decodePath(location.pathname)}) is not available.`,
      actionLabel: 'Back',
      action: () => navigate(-1),
    };
  });

  const icons = ['pixelarticons:fire', 'pixelarticons:siren-sharp', 'pixelarticons:skull', 'pixelarticons:square-alert'];

  const pickRandomIcon = () => {
    const rndIndex = Math.floor(Math.random() * icons.length);
    return icons[rndIndex];
  };

  return (
    <div class='flex flex-col gap-4 items-center justify-center h-dvh w-dvw bg-primary-surface'>
      <div class='flex flex-col gap-4 p-4 min-w-[40%] bg-primary-surface'>
        <Icon icon={pickRandomIcon()} class='text-8xl w-fit text-zinc-700' />
        <Heading1>Oops!</Heading1>
        <p class='archivo italic opacity-75'>{model().message}</p>
        <button class='w-fit' onClick={model().action}>
          {model().actionLabel}
        </button>
      </div>
    </div>
  );
}

export default AppError;
