import type { AppBootstrap } from '~/bindings';

export const HOME_ROUTE = '/home';
export const INIT_LOGIN_ROUTE = '/init_login';
export const HOME_ERROR_ROUTE = '/home/error';

export type HomeErrorKind = 'no_network' | 'connection_error' | 'reauth_required';

export function toHomeErrorRoute(kind: HomeErrorKind) {
  return `${HOME_ERROR_ROUTE}?kind=${encodeURIComponent(kind)}`;
}

export function resolveBootstrapRoute(bootstrap: AppBootstrap) {
  switch (bootstrap.restoreStatus) {
    case 'restored':
      return HOME_ROUTE;
    case 'network_error':
      return toHomeErrorRoute('no_network');
    case 'connection_error':
      return toHomeErrorRoute('connection_error');
    case 'reauth_required':
      return toHomeErrorRoute('reauth_required');
    case 'none':
      return bootstrap.activeSession ? HOME_ROUTE : INIT_LOGIN_ROUTE;
  }
}
