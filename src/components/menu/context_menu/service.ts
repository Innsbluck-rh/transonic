// Write global signals (like open/close) and functions of the context menus which called from outside components.
// This will allow components to refresh its layout properly on hot reload (otherwise it doesn't.)

import { createSignal } from 'solid-js';
import type { MenuItem } from '~/features/menu/types';

export interface ContextMenuState {
  items: MenuItem[];
  anchorX: number;
  anchorY: number;
}

const [contextMenu, setContextMenu] = createSignal<ContextMenuState | null>(null);

export { contextMenu };

export function openContextMenu(items: MenuItem[], x: number, y: number) {
  setContextMenu({ items, anchorX: x, anchorY: y });
}

export function closeContextMenu() {
  setContextMenu(null);
}
