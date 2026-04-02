import interact from 'interactjs';
import { Component } from 'solid-js';
import IndexContent from './IndexContent';

const IndexSideBar: Component = () => {
  interact('.index-sidebar').resizable({
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

  return (
    <div class='index-sidebar border-primary-border flex w-56 max-w-96 min-w-40 flex-col border-r'>
      <IndexContent />
    </div>
  );
};

export default IndexSideBar;
