import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, createSignal, For, JSX } from 'solid-js';

import styles from './VolumeSlider.module.css';

interface SeekSliderProps {
  value?: number;
  max?: number;
  disabled?: boolean;
  onPreview?: (value: number | null) => void;
  onCommit?: (value: number) => void;
}

function clampSliderValue(value: number, max: number) {
  if (max <= 0) {
    return 0;
  }

  return Math.min(Math.max(value, 0), max);
}

// TODO: SeekSlider.tsxからとってきた不要な処理とprops指定を消す
// TODO:
// TODO: Use actual Volume set commands instead of onCommit (and remove other props that arent needed)
const VolumeSlider: Component<SeekSliderProps> = (props) => {
  let sliderRef: HTMLDivElement | undefined;
  const [dragValue, setDragValue] = createSignal<number | null>(null);
  const [activePointerId, setActivePointerId] = createSignal<number | null>(null);

  const maxValue = createMemo(() => Math.max(0, props.max ?? 0));
  const displayedValue = createMemo(() => {
    const nextValue = dragValue() ?? props.value ?? 0;
    return clampSliderValue(nextValue, maxValue());
  });

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

  const volumeIcon = createMemo(() => {
    const nextVolume = displayedValue();
    if (nextVolume <= 0) {
      return 'material-symbols:volume-off';
    } else if (nextVolume < 50) {
      return 'material-symbols:volume-down';
    } else {
      return 'material-symbols:volume-up';
    }
  });

  const indicators = createMemo<{ on: boolean; value: number; percentage: number }[]>(() => {
    const values = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

    const dispValue = displayedValue();

    return values.map((v, i) => {
      const isOn = dispValue >= v;
      let percentage = 0;
      if (isOn) {
        // ex: current=74, i=6, v=70
        const diffFromCurrentValue = dispValue - v; // 74 - 70 = 4
        const diffFromNextValue = values[i + 1] - v; // 80 - 70 = 10
        // calcPercent = round((4/10) * 100) = round(0.4 * 100) = 40%
        const calcPercent = Math.ceil((diffFromCurrentValue / diffFromNextValue) * 100);
        percentage = Math.max(0, Math.min(100, calcPercent));
      }

      return {
        on: isOn,
        percentage,
        value: v,
      };
    });
  });

  const [mutePrevValue, setMutePrevValue] = createSignal<number | undefined>(undefined);

  const [rootHovered, setRootHovered] = createSignal<boolean | undefined>(undefined);

  return (
    <div
      class='flex w-auto shrink-0 items-center overflow-x-hidden p-1'
      onPointerEnter={() => {
        setRootHovered(true);
      }}
      onPointerLeave={() => {
        setRootHovered(false);
      }}
      onClick={(event) => event.stopPropagation()}
    >
      <Icon
        icon={volumeIcon()}
        class='text-primary-text scale-125 cursor-pointer'
        onClick={(e) => {
          // when muted
          if (displayedValue() === 0) {
            // try to retrieve prev value, set 100 if nothing saved yet
            const nextValue = mutePrevValue();
            props.onCommit?.(nextValue === undefined ? 100 : nextValue);
            setMutePrevValue(undefined);
          } else {
            // if unmuted, save value and mute
            setMutePrevValue(displayedValue());
            props.onCommit?.(0);
          }
        }}
      />

      <div
        // 2*10 + 4*9 = 56
        class={`relative flex h-8 touch-none items-center gap-[4px] ${styles.defaultSliderState} ${rootHovered() && styles.sliderAppear} ${rootHovered() === false && styles.sliderDisappear}`}
        onClick={(e) => e.stopPropagation()}
        title={displayedValue().toString()}
      >
        <div
          ref={sliderRef}
          class={`absolute flex h-5 w-[56px] cursor-pointer touch-none flex-row items-end gap-[4px]`}
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerCancel={handlePointerCancel}
          onClick={(e) => e.stopPropagation()}
        >
          <For each={indicators()}>
            {(indicator) => (
              <div
                class='relative w-[2px]'
                style={{
                  height: `${indicator.value}%`,
                }}
              >
                <div class='bg-secondary-text absolute inset-0' />
                <div
                  class='bg-primary-text absolute inset-0'
                  style={{
                    opacity: `${indicator.percentage}%`,
                  }}
                />
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
};

export default VolumeSlider;
