import { useLocation, useNavigate, type NavigateOptions } from '@solidjs/router';
import { setSPNavStore, type NavDirection } from '~/stores/SPNavigationStore';

function setNavDirection(direction: NavDirection) {
  setSPNavStore('direction', direction);
  document.documentElement.dataset.navDirection = direction;
}

function clearNavDirection() {
  setSPNavStore('direction', null);
  delete document.documentElement.dataset.navDirection;
}

/**
 * SP専用のナビゲーションフック。
 * SP画面内での遷移にView Transitions APIを使ったスライドアニメーションを付与する。
 * SP以外のルートからの呼び出しでは通常のnavigateにフォールバックする。
 */
export function useSPNavigate() {
  const navigate = useNavigate();
  const location = useLocation();

  return (to: string | number, options?: Partial<NavigateOptions>) => {
    // @solidjs/router の navigate はオーバーロード定義のため as any でキャスト
    const nav = (t: string | number) => (navigate as (to: string | number, opts?: Partial<NavigateOptions>) => void)(t, options);

    if (!location.pathname.startsWith('/sp')) {
      nav(to);
      return;
    }

    const direction: NavDirection = typeof to === 'number' && to < 0 ? 'backward' : 'forward';
    setNavDirection(direction);

    if (document.startViewTransition) {
      const transition = document.startViewTransition(() => nav(to));
      void transition.finished.then(clearNavDirection);
    } else {
      nav(to);
    }
  };
}
