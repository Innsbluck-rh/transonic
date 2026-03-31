import type { AppBootstrap } from '~/bindings';

export const HOME_ROUTE = '/home';
export const INIT_LOGIN_ROUTE = '/init_login';
export const HOME_NO_NETWORK_ROUTE = '/home/no_network';
export const HOME_CONNECTION_ERROR_ROUTE = '/home/connection_error';

export function resolveBootstrapRoute(bootstrap: AppBootstrap) {
  switch (bootstrap.restoreStatus) {
    case 'restored':
      return HOME_ROUTE;
    case 'network_error':
      return HOME_NO_NETWORK_ROUTE;
    case 'connection_error':
    case 'reauth_required':
      return HOME_CONNECTION_ERROR_ROUTE;
    case 'none':
      return bootstrap.activeSession ? HOME_ROUTE : INIT_LOGIN_ROUTE;
  }
}

export function isHomeErrorRoute(pathname: string) {
  return pathname === HOME_NO_NETWORK_ROUTE || pathname === HOME_CONNECTION_ERROR_ROUTE;
}
