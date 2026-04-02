import { Component, createEffect, createMemo, createSignal, JSX, onCleanup, onMount, Show, splitProps } from 'solid-js';

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
    <p {...rest} ref={rootRef} class={`marquee-paragraph ${local.class ?? ''}`.trim()}>
      <span class='marquee-paragraph__viewport'>
        <span
          class='marquee-paragraph__track'
          classList={{ 'marquee-paragraph__track--animate': isOverflowing() }}
          style={{
            '--marquee-gap': `${gapPx()}px`,
            '--marquee-distance': `${distancePx()}px`,
            '--marquee-duration': durationSeconds(),
          }}
        >
          <span ref={contentRef} class='marquee-paragraph__content'>
            {local.text}
          </span>
          <Show when={isOverflowing()}>
            <span class='marquee-paragraph__content marquee-paragraph__content--duplicate' aria-hidden='true'>
              {local.text}
            </span>
          </Show>
        </span>
      </span>
    </p>
  );
};

export default MarqueeParagraph;
