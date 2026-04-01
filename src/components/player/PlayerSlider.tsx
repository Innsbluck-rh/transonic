import { Component, createMemo, createSignal, JSX } from 'solid-js';

interface PlayerSliderProps {
  progressRatio?: number;
  disabled?: boolean;
  onSeekPreview?: (ratio: number | null) => void;
  onSeekCommit?: (ratio: number) => void;
}

const PlayerSlider: Component<PlayerSliderProps> = (props) => {
  let sliderRef: HTMLDivElement | undefined;
  const [dragRatio, setDragRatio] = createSignal<number | null>(null);
  const [activePointerId, setActivePointerId] = createSignal<number | null>(null);

  const clampRatio = (ratio: number) => Math.min(Math.max(ratio, 0), 1);

  const ratioFromClientX = (clientX: number) => {
    const slider = sliderRef;
    if (!slider) {
      return 0;
    }

    const rect = slider.getBoundingClientRect();
    if (rect.width <= 0) {
      return 0;
    }

    return clampRatio((clientX - rect.left) / rect.width);
  };

  const ratio = createMemo(() => clampRatio(dragRatio() ?? props.progressRatio ?? 0));
  const progressWidth = () => `${ratio() * 100}%`;
  const isDragging = () => activePointerId() !== null;

  const updateDragRatio = (clientX: number) => {
    const nextRatio = ratioFromClientX(clientX);
    setDragRatio(nextRatio);
    props.onSeekPreview?.(nextRatio);
    return nextRatio;
  };

  const finishDrag = (nextRatio: number | null, shouldCommit: boolean) => {
    setActivePointerId(null);
    setDragRatio(null);
    props.onSeekPreview?.(null);

    if (shouldCommit && nextRatio !== null) {
      props.onSeekCommit?.(nextRatio);
    }
  };

  const handlePointerDown: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (props.disabled) {
      return;
    }

    event.preventDefault();
    updateDragRatio(event.clientX);
    setActivePointerId(event.pointerId);
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const handlePointerMove: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (props.disabled || activePointerId() !== event.pointerId) {
      return;
    }

    updateDragRatio(event.clientX);
  };

  const handlePointerUp: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (activePointerId() !== event.pointerId) {
      return;
    }

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    finishDrag(updateDragRatio(event.clientX), true);
  };

  const handlePointerCancel: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (activePointerId() !== event.pointerId) {
      return;
    }

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    finishDrag(null, false);
  };

  return (
    <div
      ref={sliderRef}
      class='group relative w-full h-2 touch-none'
      classList={{ 'cursor-pointer': !props.disabled, 'cursor-default': !!props.disabled }}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerCancel}
    >
      <div class='absolute top-1/2 left-0 w-full h-2 -translate-y-1/2'></div>
      {/* background */}
      <div class='absolute top-1/2 left-0 w-full h-0.5 -translate-y-1/2 bg-secondary-border'></div>
      {/* progress */}
      <div class='absolute top-1/2 left-0 h-0.5 -translate-y-1/2 bg-blue-500' style={{ width: progressWidth() }}></div>
      {/* handle */}
      <div
        class='absolute top-1/2 origin-center w-2 h-2 rounded-full bg-blue-500 transition-opacity translate-x-[-50%] -translate-y-1/2 opacity-0 group-hover:opacity-100'
        classList={{ 'opacity-100': isDragging() }}
        style={{ left: progressWidth() }}
      ></div>
    </div>
  );
};

export default PlayerSlider;
