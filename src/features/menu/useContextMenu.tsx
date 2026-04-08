import { JSX } from 'solid-js';
import { isSP } from '~/utils/isSP';
import { showContextMenu } from './showMenu';
import { MenuItem } from './types';

function useContextMenu(
  items: MenuItem[]
):
  | Pick<JSX.HTMLAttributes<HTMLElement>, 'onTouchStart' | 'onMouseDown' | 'onTouchEnd' | 'onMouseUp' | 'onMouseLeave'>
  | Pick<JSX.HTMLAttributes<HTMLElement>, 'onContextMenu'> {
  if (isSP()) {
    let timer: number | undefined;
    function startLongPress(e: TouchEvent | MouseEvent) {
      timer = setTimeout(() => {
        e.preventDefault();
        showContextMenu(e, items);
      }, 800);
    }

    function cancelLongPress(e: TouchEvent | MouseEvent) {
      clearTimeout(timer); // タイマーをクリア
    }

    return {
      onTouchStart: startLongPress,
      onMouseDown: startLongPress,
      onTouchEnd: cancelLongPress,
      onMouseUp: cancelLongPress,
      onMouseLeave: cancelLongPress,
    };
  } else {
    return {
      onContextMenu: (e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        showContextMenu(e, items);
      },
    };
  }
}

export default useContextMenu;
