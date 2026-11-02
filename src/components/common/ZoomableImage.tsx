import Panzoom, { PanzoomEventDetail, PanzoomObject } from '@panzoom/panzoom';
import { Component, createMemo, createSignal, onCleanup, onMount } from 'solid-js';

/** How far past actual size (one image pixel per CSS pixel) zooming may go. */
const DEFAULT_MAX_SCALE_OF_ACTUAL_SIZE = 2;
const DEFAULT_WHEEL_ZOOM_SPEED = 0.002;

const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

/**
 * Scale 1 is the image fitted into the viewport (never enlarged).
 * Wheel / pinch zooms, a click toggles actual size, touch drags pan,
 * and with a mouse the cursor position across the viewport selects the visible part of a zoomed image.
 */
const ZoomableImage: Component<{ src: string; maxScale?: number; wheelZoomSpeed?: number }> = (props) => {
  let viewport!: HTMLDivElement;
  let img!: HTMLImageElement;
  let panzoom: PanzoomObject | undefined;
  const [fitSize, setFitSize] = createSignal<{ width: number; height: number }>();
  const [zoomed, setZoomed] = createSignal(false);

  const maxScale = createMemo(() => props.maxScale ?? DEFAULT_MAX_SCALE_OF_ACTUAL_SIZE);
  const wheelZoomSpeed = createMemo(() => props.wheelZoomSpeed ?? DEFAULT_WHEEL_ZOOM_SPEED);

  const actualScale = () => {
    const size = fitSize();
    return size ? img.naturalWidth / size.width : 1;
  };

  const fit = () => {
    const { naturalWidth, naturalHeight } = img;
    if (!naturalWidth || !naturalHeight) return;
    const ratio = Math.min(1, viewport.clientWidth / naturalWidth, viewport.clientHeight / naturalHeight);
    setFitSize({ width: naturalWidth * ratio, height: naturalHeight * ratio });
    panzoom?.setOptions({ maxScale: actualScale() * maxScale() });
    panzoom?.reset({ animate: false });
  };

  /**
   * How far the image may move from the center: it keeps covering the viewport along an axis where it is larger,
   * and stays centered where it is not. In panzoom's translate units, which are applied after scaling (unscaled px).
   * Computed here instead of panzoom's `contain`, which measures the element before the pending transform is applied.
   */
  const panLimit = () => {
    const size = fitSize();
    const scale = panzoom?.getScale() ?? 1;
    if (!size) return { x: 0, y: 0 };
    return {
      x: Math.max(0, size.width * scale - viewport.clientWidth) / 2 / scale,
      y: Math.max(0, size.height * scale - viewport.clientHeight) / 2 / scale,
    };
  };

  const clampPan = () => {
    if (!panzoom) return;
    const limit = panLimit();
    const { x, y } = panzoom.getPan();
    const clampedX = Math.min(limit.x, Math.max(-limit.x, x));
    const clampedY = Math.min(limit.y, Math.max(-limit.y, y));
    if (clampedX !== x || clampedY !== y) panzoom.pan(clampedX, clampedY, { animate: false, force: true });
  };

  const followPointer = (point: { clientX: number; clientY: number }) => {
    if (!panzoom) return;
    const limit = panLimit();
    const rect = viewport.getBoundingClientRect();
    const fx = clamp01((point.clientX - rect.left) / rect.width);
    const fy = clamp01((point.clientY - rect.top) / rect.height);
    panzoom.pan((1 - 2 * fx) * limit.x, (1 - 2 * fy) * limit.y, { animate: false });
  };

  const toggleActualSize = (e: MouseEvent) => {
    e.stopPropagation();
    if (!panzoom) return;
    if (panzoom.getScale() > 1) {
      panzoom.reset({ animate: false });
    } else if (e instanceof PointerEvent && e.pointerType !== 'mouse') {
      panzoom.zoomToPoint(actualScale(), e, { animate: false });
      clampPan();
    } else {
      panzoom.zoom(actualScale(), { animate: false });
      followPointer(e);
    }
  };

  const onWheel = (e: WheelEvent) => {
    if (!panzoom) return;
    e.preventDefault();
    const delta = Math.max(-100, Math.min(100, e.deltaMode === WheelEvent.DOM_DELTA_LINE ? e.deltaY * 16 : e.deltaY));
    panzoom.zoom(panzoom.getScale() * Math.exp(-delta * wheelZoomSpeed()), { animate: false });
    followPointer(e);
  };

  // panzoom's own pointer handling is bound for touch / pen only; a mouse never drags.
  const onPointerDown = (e: PointerEvent) => {
    if (e.pointerType !== 'mouse') panzoom?.handleDown(e);
  };
  const onPointerMove = (e: PointerEvent) => {
    if (e.pointerType === 'mouse') {
      followPointer(e);
    } else {
      panzoom?.handleMove(e);
      clampPan();
    }
  };
  const onPointerUp = (e: PointerEvent) => {
    if (e.pointerType !== 'mouse') panzoom?.handleUp(e);
  };
  const onPanzoomChange = (e: Event) => {
    setZoomed((e as CustomEvent<PanzoomEventDetail>).detail.scale > 1);
  };

  onMount(() => {
    panzoom = Panzoom(img, {
      noBind: true,
      cursor: '',
      overflow: 'visible',
      minScale: 1,
      panOnlyWhenZoomed: true,
    });
    img.addEventListener('panzoomchange', onPanzoomChange);
    viewport.addEventListener('wheel', onWheel, { passive: false });
    viewport.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('pointermove', onPointerMove);
    document.addEventListener('pointerup', onPointerUp);
    document.addEventListener('pointercancel', onPointerUp);
    const resizeObserver = new ResizeObserver(fit);
    resizeObserver.observe(viewport);

    onCleanup(() => {
      resizeObserver.disconnect();
      document.removeEventListener('pointermove', onPointerMove);
      document.removeEventListener('pointerup', onPointerUp);
      document.removeEventListener('pointercancel', onPointerUp);
      panzoom?.destroy();
      panzoom = undefined;
    });
  });

  return (
    <div ref={viewport} class='flex size-full min-h-0 min-w-0 touch-none items-center justify-center'>
      <div
        class='group pointer-events-auto relative shadow-lg'
        classList={{ 'cursor-zoom-in': !zoomed() && actualScale() > 1, 'cursor-zoom-out': zoomed() }}
        onClick={toggleActualSize}
      >
        <img
          ref={img}
          class='max-w-none'
          classList={{ invisible: !fitSize() }}
          style={fitSize() ? { width: `${fitSize()!.width}px`, height: `${fitSize()!.height}px` } : undefined}
          src={props.src}
          loading='lazy'
          decoding='async'
          onLoad={fit}
        />
      </div>
    </div>
  );
};

export default ZoomableImage;
