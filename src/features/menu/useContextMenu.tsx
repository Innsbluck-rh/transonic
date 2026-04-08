import { JSX } from 'solid-js';
import { isSP } from '~/utils/isSP';
import { showContextMenu } from './showMenu';
import { MenuItem } from './types';

function useContextMenu(
  items: MenuItem[]
):
  | Pick<JSX.HTMLAttributes<HTMLElement>, 'onTouchStart' | 'onTouchMove' | 'onTouchEnd' | 'onTouchCancel' | 'onContextMenu'>
  | Pick<JSX.HTMLAttributes<HTMLElement>, 'onContextMenu'> {
  if (isSP()) {
    let timer: number | undefined;

    const cancelLongPress = () => clearTimeout(timer);

    return {
      onTouchStart: (e: TouchEvent) => {
        timer = setTimeout(() => showContextMenu(e, items), 400);
      },
      onTouchMove: cancelLongPress,
      onTouchEnd: cancelLongPress,
      onTouchCancel: cancelLongPress,
      // AndroidのデフォルトcontextmenuをpreventしてSPBottomContextMenuに一本化する
      onContextMenu: (e: MouseEvent) => e.preventDefault(),
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
