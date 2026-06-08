import interact from 'interactjs';
import QueueSidebarSection from './queue/QueueSidebarSection';

const RightSideBar = () => {
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
    <div class='bg-primary-plane queue-sidebar border-primary-border flex min-h-0 w-80 max-w-[50%] min-w-56 flex-col gap-0 overflow-y-hidden border-l'>
      <QueueSidebarSection />
      {/*<div class='bg-primary-border h-px w-full' />
      <ConnectSidebarSection />*/}
    </div>
  );
};

export default RightSideBar;
