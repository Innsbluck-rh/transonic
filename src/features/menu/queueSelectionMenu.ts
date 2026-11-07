import { clearQueueSelection, removeSelectedQueueRows, selectAllQueueRows } from '~/stores/QueueSelectionStore';
import type { MenuItem } from './types';

/**
 * Right-click menu of the queue while rows are selected. Its actions read the selection when
 * clicked: the selection follows queue changes made while the menu is open, a copy would not.
 */
export function buildQueueSelectionMenuItems(): MenuItem[] {
  return [
    {
      icon: 'icon-[material-symbols--playlist-remove]',
      label: 'Remove from Queue',
      onClick: () => removeSelectedQueueRows(),
    },
    {
      icon: 'icon-[material-symbols--select-all]',
      label: 'Select All',
      onClick: selectAllQueueRows,
    },
    {
      icon: 'icon-[material-symbols--close]',
      label: 'Deselect',
      onClick: clearQueueSelection,
    },
  ];
}
