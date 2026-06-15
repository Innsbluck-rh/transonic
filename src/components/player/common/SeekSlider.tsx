import { Component, createMemo, createSignal, JSX } from 'solid-js';

interface SeekSliderProps {
  value?: number;
  max?: number;
  disabled?: boolean;
  onPreview?: (value: number | null) => void;
  onCommit?: (value: number) => void;
  rootClass?: string;
  hitAreaClass?: string;
  trackClass?: string;
  progressClass?: string;
  handleClass?: string;
}

function clampSliderValue(value: number, max: number) {
  if (max <= 0) {
    return 0;
  }

  return Math.min(Math.max(value, 0), max);
}

function sliderWidth(value: number, max: number) {
  if (max <= 0) {
    return '0%';
  }

  return `${(value / max) * 100}%`;
}

const SeekSlider: Component<SeekSliderProps> = (props) => {
  let sliderRef: HTMLDivElement | undefined;
  const [dragValue, setDragValue] = createSignal<number | null>(null);
  const [activePointerId, setActivePointerId] = createSignal<number | null>(null);

  const maxValue = createMemo(() => Math.max(0, props.max ?? 0));
  const displayedValue = createMemo(() => {
    const nextValue = dragValue() ?? props.value ?? 0;
    return clampSliderValue(nextValue, maxValue());
  });
  const progressWidth = createMemo(() => sliderWidth(displayedValue(), maxValue()));
  const isDragging = createMemo(() => activePointerId() !== null);

  const valueFromClientX = (clientX: number) => {
    const slider = sliderRef;
    const max = maxValue();
    if (!slider || max <= 0) {
      return 0;
    }

    const rect = slider.getBoundingClientRect();
    if (rect.width <= 0) {
      return 0;
    }

    const ratio = Math.min(Math.max((clientX - rect.left) / rect.width, 0), 1);
    return Math.round(max * ratio);
  };

  const updateDragValue = (clientX: number) => {
    const nextValue = valueFromClientX(clientX);
    setDragValue(nextValue);
    props.onPreview?.(nextValue);
    return nextValue;
  };

  const finishDrag = (nextValue: number | null, shouldCommit: boolean) => {
    setActivePointerId(null);
    setDragValue(null);
    props.onPreview?.(null);

    if (shouldCommit && nextValue !== null) {
      props.onCommit?.(nextValue);
    }
  };

  const handlePointerDown: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (props.disabled) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    updateDragValue(event.clientX);
    setActivePointerId(event.pointerId);
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const handlePointerMove: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (props.disabled || activePointerId() !== event.pointerId) {
      return;
    }
    event.stopPropagation();

    updateDragValue(event.clientX);
  };

  const handlePointerUp: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (activePointerId() !== event.pointerId) {
      return;
    }

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    finishDrag(updateDragValue(event.clientX), true);
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
      class={props.rootClass ?? 'group relative h-2 w-full touch-none'}
      classList={{ 'cursor-pointer': !props.disabled, 'cursor-default': !!props.disabled }}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerCancel}
      onClick={(e) => e.stopPropagation()}
    >
      <div class={props.hitAreaClass ?? 'absolute top-1/2 left-0 h-2 w-full -translate-y-1/2'}></div>
      <div class={props.trackClass ?? 'bg-secondary-border absolute top-1/2 left-0 h-0.5 w-full -translate-y-1/2'}></div>
      <div class={props.progressClass ?? 'bg-accent absolute top-1/2 left-0 h-0.5 -translate-y-1/2'} style={{ width: progressWidth() }}></div>
      <div
        class={
          props.handleClass ??
          'bg-accent absolute top-1/2 h-2 w-2 origin-center translate-x-[-50%] -translate-y-1/2 rounded-full opacity-0 transition-opacity group-hover:opacity-100'
        }
        classList={{ 'opacity-100': isDragging() }}
        style={{ left: progressWidth() }}
      ></div>
    </div>
  );
};

export default SeekSlider;
