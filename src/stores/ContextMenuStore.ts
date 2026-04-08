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
