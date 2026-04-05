import { Menu, MenuItem } from '@tauri-apps/api/menu';

export interface MenuItemDef {
  label: string;
  action: () => void;
  enabled?: boolean;
}

export async function showContextMenu(e: MouseEvent, items: MenuItemDef[]): Promise<void> {
  e.preventDefault();

  const menuItems = await Promise.all(
    items.map((item) =>
      MenuItem.new({
        text: item.label,
        enabled: item.enabled ?? true,
        action: () => item.action(),
      })
    )
  );

  const menu = await Menu.new({ items: menuItems });
  await menu.popup();
}
