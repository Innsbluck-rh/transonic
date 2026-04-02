import interact from 'interactjs';
import { Component } from 'solid-js';
import QueueContent from './QueueContent';

const QueueSideBar: Component = () => {
  interact('.queue-sidebar').resizable({
    edges: { left: true },
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
    <div class='queue-sidebar bg-primary-plane border-primary-border flex w-56 max-w-96 min-w-40 flex-col border-l'>
      <div class='border-secondary-border flex h-6 w-full flex-row items-center border-b px-1'>
        <p class='archivo text-secondary-text text-[10px]'>Playback Queue</p>
      </div>
      <QueueContent />
    </div>
  );
};

export default QueueSideBar;
