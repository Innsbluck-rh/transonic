import { useLocation, useNavigate, type NavigateOptions } from '@solidjs/router';
import { closePlayerBar } from '~/components/sp/player/SPExpandablePlayerBar';
import { isSP } from '~/utils/isSP';
import { resolveSPBrowseRoute } from './routes';

type NavDirection = 'forward' | 'backward';

function setNavDirection(direction: NavDirection) {
  document.documentElement.dataset.navDirection = direction;
}

function clearNavDirection() {
  delete document.documentElement.dataset.navDirection;
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

function insertSPBrowseIndexHistoryEntry() {
  const browseRoot = resolveSPBrowseRoute();

  window.history.pushState({ _depth: window.history.length }, '', browseRoot);
}

/**
 * SP専用のナビゲーションフック。
 * SP画面内での遷移にView Transitions APIを使ったフェードアニメーションを付与する。
 * SP以外のプラットフォームからの呼び出しでは通常のnavigateにフォールバックする。
 *
 * SPPlayerバーが展開中の場合、遷移前にバーを閉じる。
 * これにより、View Transitionのアニメーション擬似要素がプレイヤーの上に
 * 描画されてしまうz軸の問題を回避する。
 */
export function useSPNavigate() {
  const navigate = useNavigate();
  const location = useLocation();

  return (to: string | number, options?: Partial<NavigateOptions>) => {
    // @solidjs/router の navigate はオーバーロード定義のため as any でキャスト
    const nav = (t: string | number) => (navigate as (to: string | number, opts?: Partial<NavigateOptions>) => void)(t, options);

    if (!isSP()) {
      nav(to);
      return;
    }

    // SPPlayerバーが展開中なら閉じてからナビゲーションを行う
    closePlayerBar(true);

    const direction: NavDirection = typeof to === 'number' && to < 0 ? 'backward' : 'forward';
    setNavDirection(direction);

    const currentPathname = normalizePathname(location.pathname);
    const targetPathname = typeof to === 'string' ? normalizePathname(to) : null;
    const shouldInsertBrowseIndex =
      targetPathname !== null && !options?.replace && isSPBrowseDetailPath(targetPathname) && !isSPBrowsePath(currentPathname);

    const navigateWithBrowseIndex = () => {
      if (shouldInsertBrowseIndex) {
        insertSPBrowseIndexHistoryEntry();
      }
      nav(to);
    };

    if (document.startViewTransition) {
      const transition = document.startViewTransition(navigateWithBrowseIndex);
      void transition.finished.then(clearNavDirection);
    } else {
      navigateWithBrowseIndex();
    }
  };
}
