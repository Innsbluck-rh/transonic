import type { AppBootstrap } from '~/bindings';
import { HOME_ERROR_ROUTE, HOME_ROUTE, INIT_LOGIN_ROUTE, resolveHomeRoute } from '~/features/navigation/routes';

export { HOME_ERROR_ROUTE, HOME_ROUTE, INIT_LOGIN_ROUTE };

export type HomeErrorKind = 'no_network' | 'connection_error' | 'reauth_required';

export function toHomeErrorRoute(kind: HomeErrorKind) {
  return `${HOME_ERROR_ROUTE}?kind=${encodeURIComponent(kind)}`;
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
      return bootstrap.activeSession ? homeRoute : INIT_LOGIN_ROUTE;
  }
}
