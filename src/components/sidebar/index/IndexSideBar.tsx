import interact from 'interactjs';
import { Component } from 'solid-js';
import IndexContent from './IndexContent';

const IndexSideBar: Component = () => {
  interact('.index-sidebar').resizable({
    edges: { right: true },
    listeners: {
      move: function (event) {
        let { x, y } = event.target.dataset;

        x = (parseFloat(x) || 0) + event.deltaRect.left;
        y = (parseFloat(y) || 0) + event.deltaRect.top;

        // 要素のサイズと位置を更新
        Object.assign(event.target.style, {
          width: `${event.rect.width}px`,
          height: `${event.rect.height}px`,
          transform: `translate(${x}px, ${y}px)`,
        });

        Object.assign(event.target.dataset, { x, y });
      },
    },
  });

  return (
    <div class='index-sidebar flex flex-col min-w-40 w-56 border-r border-primary-border resize-x overflow-auto'>
      <IndexContent />
    </div>
  );
};

export default IndexSideBar;
