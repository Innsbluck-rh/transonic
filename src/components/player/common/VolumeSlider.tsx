import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, createSignal, For, JSX } from 'solid-js';

import styles from './VolumeSlider.module.css';

interface VolumeSliderProps {
  volume?: number;
  disabled?: boolean;
  onVolumeInput?: (volume: number) => void;
  onVolumeCommit?: (volume: number) => void;
}

function clampVolume(value: number) {
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.min(Math.max(value, 0), 1);
}

function volumePercentage(volume: number) {
  return Math.round(clampVolume(volume) * 100);
}

const VolumeSlider: Component<VolumeSliderProps> = (props) => {
  let sliderRef: HTMLDivElement | undefined;
  let lastInputVolume: number | null = null;
  const [dragVolume, setDragVolume] = createSignal<number | null>(null);
  const [activePointerId, setActivePointerId] = createSignal<number | null>(null);

  const displayedVolume = createMemo(() => clampVolume(dragVolume() ?? props.volume ?? 0));
  const displayedPercentage = createMemo(() => volumePercentage(displayedVolume()));

  const volumeFromClientX = (clientX: number) => {
    const slider = sliderRef;
    if (!slider) {
      return 0;
    }

    const rect = slider.getBoundingClientRect();
    if (rect.width <= 0) {
      return 0;
    }

    const ratio = Math.min(Math.max((clientX - rect.left) / rect.width, 0), 1);
    return Math.round(ratio * 100) / 100;
  };

  const updateDragVolume = (clientX: number) => {
    const nextValue = volumeFromClientX(clientX);
    setDragVolume(nextValue);
    if (lastInputVolume !== nextValue) {
      lastInputVolume = nextValue;
      props.onVolumeInput?.(nextValue);
    }
    return nextValue;
  };

  const finishDrag = (nextVolume: number | null) => {
    setActivePointerId(null);
    setDragVolume(null);
    lastInputVolume = null;

    if (nextVolume !== null) {
      props.onVolumeCommit?.(nextVolume);
    }
  };

  const handlePointerDown: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (props.disabled) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    updateDragVolume(event.clientX);
    setActivePointerId(event.pointerId);
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const handlePointerMove: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (props.disabled || activePointerId() !== event.pointerId) {
      return;
    }
    event.stopPropagation();

    updateDragVolume(event.clientX);
  };

  const handlePointerUp: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (activePointerId() !== event.pointerId) {
      return;
    }

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    finishDrag(updateDragVolume(event.clientX));
  };

  const handlePointerCancel: JSX.EventHandlerUnion<HTMLDivElement, PointerEvent> = (event) => {
    if (activePointerId() !== event.pointerId) {
      return;
    }

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    finishDrag(null);
  };

  const volumeIcon = createMemo(() => {
    const nextVolume = displayedPercentage();
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

    const dispValue = displayedPercentage();

    return values.map((v, i) => {
      const isOn = dispValue >= v;
      let percentage = 0;
      if (isOn) {
        if (i === values.length - 1) {
          percentage = 100;
          return {
            on: isOn,
            percentage,
            value: v,
          };
        }

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

  const commitVolume = (nextVolume: number) => {
    const clampedVolume = clampVolume(nextVolume);
    props.onVolumeInput?.(clampedVolume);
    props.onVolumeCommit?.(clampedVolume);
  };

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
          e.stopPropagation();
          if (props.disabled) {
            return;
          }

          // when muted
          if (displayedVolume() === 0) {
            // try to retrieve prev value, set 100 if nothing saved yet
            const nextValue = mutePrevValue();
            commitVolume(nextValue === undefined ? 1 : nextValue);
            setMutePrevValue(undefined);
          } else {
            // if unmuted, save value and mute
            setMutePrevValue(displayedVolume());
            commitVolume(0);
          }
        }}
      />

      <div
        // 2*10 + 4*9 = 56
        class={`relative flex h-8 touch-none items-center gap-[4px] ${styles.defaultSliderState} ${rootHovered() && styles.sliderAppear} ${rootHovered() === false && styles.sliderDisappear}`}
        onClick={(e) => e.stopPropagation()}
        title={displayedPercentage().toString()}
      >
        <div
          ref={sliderRef}
          class={`absolute flex h-5 w-[56px] cursor-pointer touch-none flex-row items-end gap-[4px]`}
          role='slider'
          aria-label='Volume'
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={displayedPercentage()}
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
