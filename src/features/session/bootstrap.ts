import type { AppBootstrap } from '~/bindings';
import { resolveHomeRoute } from '~/features/navigation/routes';

export type HomeErrorKind = 'no_network' | 'connection_error' | 'reauth_required';

export function toHomeErrorRoute(kind: HomeErrorKind) {
  return `/home/error?kind=${encodeURIComponent(kind)}`;
}

export function resolveBootstrapRoute(bootstrap: AppBootstrap) {
  const homeRoute = resolveHomeRoute();

  switch (bootstrap.restoreStatus) {
    case 'restored':
      return homeRoute;
    case 'network_error':
      return toHomeErrorRoute('no_network');
    case 'connection_error':
      return toHomeErrorRoute('connection_error');
    case 'reauth_required':
      return toHomeErrorRoute('reauth_required');
    case 'none':
      return bootstrap.activeSession ? homeRoute : '/init_login';
  }
}
