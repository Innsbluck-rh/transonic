import interact from 'interactjs';
import { Component } from 'solid-js';
import QueueList from '~/components/common/list/song/QueueList';
import { usePlayback } from '~/features/playback/usePlayback';

const QueueSideBar: Component = () => {
  const { queue } = usePlayback();
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
    <div class='queue-sidebar bg-primary-plane border-primary-border flex h-full w-68 max-w-96 min-w-56 flex-col overflow-y-hidden border-l'>
      <div class='border-secondary-border flex h-6 w-full flex-row items-center border-b px-1'>
        <p class='archivo text-secondary-text text-[10px] font-black'>Playback Queue</p>
      </div>
      <div class='flex-1 overflow-y-auto'>
        <QueueList queue={queue()} />
      </div>
    </div>
  );
};

export default QueueSideBar;
