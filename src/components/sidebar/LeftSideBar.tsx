import interact from 'interactjs';
import { Component, createSignal } from 'solid-js';
import { BrowseMode } from '~/features/navigation/routes';
import BrowseItemList from '../browse/BrowseItemList';
import BrowseModeSelect from '../browse/BrowseModeSelect';

const LeftSideBar: Component = () => {
  interact('.browse-sidebar').resizable({
    edges: { right: true },
    listeners: {
      move: function (event) {
        event.preventDefault();
        event.stopImmediatePropagation();
        event.stopPropagation();

        let { x, y } = event.target.dataset;

        x = (parseFloat(x) || 0) + event.deltaRect.left;
        y = (parseFloat(y) || 0) + event.deltaRect.top;

        // 要素のサイズと位置を更新
        Object.assign(event.target.style, {
          width: `${event.rect.width}px`,
          height: `${event.rect.height}px`,
        });

        Object.assign(event.target.dataset, { x, y });
      },
    },
  });

  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Artists');

  return (
    <div class='bg-primary-plane browse-sidebar border-primary-border flex w-60 max-w-96 min-w-40 flex-col border-r'>
      <div class='relative flex size-full flex-col overflow-y-auto'>
        <div class='sticky top-0 z-1 box-border flex p-1'>
          <BrowseModeSelect
            defaultMode={browseMode()}
            onSelect={(mode) => {
              setBrowseMode(mode);
            }}
          />
        </div>
        <div class='flex flex-col'>
          <BrowseItemList mode={browseMode()} />
        </div>
      </div>
    </div>
  );
};

export default LeftSideBar;
