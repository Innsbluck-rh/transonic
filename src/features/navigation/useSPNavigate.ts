import { useLocation, useNavigate, type NavigateOptions } from '@solidjs/router';
import { closePlayerBar } from '~/components/sp/player/SPExpandablePlayerBar';
import { isSP } from '~/utils/isSP';
import { resolveSPBrowseRoute } from './routes';

type NavDirection = 'forward' | 'backward';

function setNavDirection(direction: NavDirection) {
  document.documentElement.dataset.navDirection = direction;
}

function normalizePathname(path: string) {
  return (path.split(/[?#]/, 1)[0] || '/').replace(/\/+$/, '') || '/';
}

function isSPBrowsePath(pathname: string) {
  const browseRoot = resolveSPBrowseRoute();
  return pathname === browseRoot || pathname.startsWith(`${browseRoot}/`);
}

function isSPBrowseDetailPath(pathname: string) {
  const browseRoot = resolveSPBrowseRoute();
  return pathname !== browseRoot && pathname.startsWith(`${browseRoot}/`);
}

// returns `navigate()` function (from `useNavigate()`), but including some intermediate processes for SP devices.
// (such as closing player, history arrangement (for /browse/...), and transition animations)
export function useSPNavigate() {
  const navigate = useNavigate();
  const location = useLocation();

  return (to: string | number, options?: Partial<NavigateOptions>) => {
    if (!isSP()) {
      navigate(to, options);
    } else {
      // Dismiss the player before the transition snapshot to avoid overlaying it.
      closePlayerBar(true);

      const direction: NavDirection = typeof to === 'number' && to < 0 ? 'backward' : 'forward';
      setNavDirection(direction);

      const currentPathname = normalizePathname(location.pathname);
      const targetPathname = typeof to === 'string' ? normalizePathname(to) : null;

      // On SP devices, `/sp/browse/XXX` routes have back button to back to /browse (BrowseList).
      // However, when the user opens `/sp/browse/XXXX` from **any components outside of the routes** (e.g.: the artist/album link in SPPlayer),
      //   any back buttons won't navigate `/sp/browse/XXX` to `/sp/browse` (which is kinda unexpected behavior).
      //
      // To prevent this, insert `/sp/browse` before `/browse/XXX` target when current path is not `/sp/browse` itself.
      // Example:
      // [Desktop] /home -> /browse/album/123 -(back)-> home
      // [SP]      /sp -> [/sp/browse] -> /sp/browse/albums/123 -(back)-> /sp/browse
      const shouldInsertBrowseRoot =
        targetPathname !== null && !options?.replace && isSPBrowseDetailPath(targetPathname) && !isSPBrowsePath(currentPathname);
      if (shouldInsertBrowseRoot) {
        const browseRoot = resolveSPBrowseRoute();
        window.history.pushState({ _depth: window.history.length }, '', browseRoot);
      }

      // Start with fading animation if supported.
      if (document.startViewTransition) {
        const transition = document.startViewTransition(() => navigate(to, options));
        void transition.finished.then(() => {
          delete document.documentElement.dataset.navDirection;
        });
      } else {
        navigate(to, options);
      }
    }
  };
}
