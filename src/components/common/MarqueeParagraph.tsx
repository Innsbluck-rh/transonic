import { Component, createEffect, createMemo, createSignal, JSX, onCleanup, onMount, Show, splitProps } from 'solid-js';

import styles from './MarqueeParagraph.module.css';

interface MarqueeParagraphProps extends JSX.HTMLAttributes<HTMLParagraphElement> {
  text: string;
  gapPx?: number;
  pixelsPerSecond?: number;
}

const DEFAULT_GAP_PX = 32;
const DEFAULT_PIXELS_PER_SECOND = 48;

const MarqueeParagraph: Component<MarqueeParagraphProps> = (props) => {
  const [local, rest] = splitProps(props, ['text', 'class', 'gapPx', 'pixelsPerSecond']);

  let rootRef: HTMLParagraphElement | undefined;
  let contentRef: HTMLSpanElement | undefined;

  const [containerWidth, setContainerWidth] = createSignal(0);
  const [contentWidth, setContentWidth] = createSignal(0);

  const gapPx = createMemo(() => Math.max(local.gapPx ?? DEFAULT_GAP_PX, 0));
  const pixelsPerSecond = createMemo(() => Math.max(local.pixelsPerSecond ?? DEFAULT_PIXELS_PER_SECOND, 1));
  const isOverflowing = createMemo(() => contentWidth() > containerWidth() + 1);
  const distancePx = createMemo(() => contentWidth() + gapPx());
  const durationSeconds = createMemo(() => `${distancePx() / pixelsPerSecond()}s`);

  const updateMeasurements = () => {
    setContainerWidth(rootRef?.clientWidth ?? 0);
    setContentWidth(contentRef?.scrollWidth ?? 0);
  };

  onMount(() => {
    updateMeasurements();

    if (typeof ResizeObserver === 'undefined') {
      return;
    }

    const observer = new ResizeObserver(() => {
      updateMeasurements();
    });

    if (rootRef) {
      observer.observe(rootRef);
    }
    if (contentRef) {
      observer.observe(contentRef);
    }

    onCleanup(() => observer.disconnect());
  });

  createEffect(() => {
    local.text;
    queueMicrotask(updateMeasurements);
  });

  return (
    <p {...rest} ref={rootRef} class={`max-w-full min-w-0 ${local.class ?? ''}`.trim()}>
      <span class={`block w-full overflow-hidden whitespace-nowrap ${styles.typographyInherit}`}>
        <span
          class='inline-flex min-w-max whitespace-nowrap will-change-transform'
          classList={{ [styles.trackAnimated]: isOverflowing() }}
          style={{
            '--marquee-gap': `${gapPx()}px`,
            '--marquee-distance': `${distancePx()}px`,
            '--marquee-duration': durationSeconds(),
          }}
        >
          <span ref={contentRef}>{local.text}</span>
          <Show when={isOverflowing()}>
            <span class={styles.duplicateContent} aria-hidden='true'>
              {local.text}
            </span>
          </Show>
        </span>
      </span>
    </p>
  );
};

export default MarqueeParagraph;
