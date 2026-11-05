import { clearQueueSelection, removeSelectedQueueRows, selectAllQueueRows } from '~/stores/QueueSelectionStore';
import type { MenuItem } from './types';

/** Right-click menu of the queue while rows are selected; `indices` is the selection when it opened. */
export function buildQueueSelectionMenuItems(indices: number[]): MenuItem[] {
  return [
    {
      icon: 'icon-[material-symbols--select-all]',
      label: 'Select All',
      onClick: selectAllQueueRows,
    },
    {
      icon: 'icon-[material-symbols--playlist-remove]',
      label: 'Remove from Queue',
      onClick: () => removeSelectedQueueRows(indices),
    },
    {
      icon: 'icon-[material-symbols--close]',
      label: 'Deselect',
      onClick: clearQueueSelection,
    },
  ];
}
