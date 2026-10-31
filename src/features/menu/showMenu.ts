import { openContextMenu } from '~/stores/ContextMenuStore';
import { cancelRipple } from '~/utils/ripple';
import type { MenuItem } from './types';

export function showContextMenu(e: TouchEvent | MouseEvent, items: MenuItem[]) {
  let clientX, clientY;
  if (e instanceof MouseEvent) {
    clientX = e.clientX;
    clientY = e.clientY;
  } else {
    // Access the first touch point
    clientX = e.touches[0].clientX;
    clientY = e.touches[0].clientY;
  }

  e.preventDefault();
  e.stopPropagation();
  cancelRipple();
  openContextMenu(items, clientX, clientY);
}
