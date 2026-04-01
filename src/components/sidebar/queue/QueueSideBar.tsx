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
    <div class='queue-sidebar flex flex-col min-w-40 w-56 max-w-96 bg-primary-plane border-l border-primary-border'>
      <div class='flex h-6 w-full flex-row items-center px-1 border-b border-secondary-border'>
        <p class='archivo text-[10px] text-secondary-text'>Playback Queue</p>
      </div>
      <QueueContent />
    </div>
  );
};

export default QueueSideBar;
