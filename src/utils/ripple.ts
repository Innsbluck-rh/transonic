const TARGET_SELECTOR = '.ripple, button';
const CANCEL_EVENT = 'transonic:ripple-cancel';
const MINIMUM_PRESS_MS = 180;
const FADE_MS = 160;

/** End press feedback when an application gesture (such as a long-press menu) takes over. */
export function cancelRipple() {
  document.dispatchEvent(new Event(CANCEL_EVENT));
}

/** Delegate input once; never enumerate or observe ripple targets. */
export function initializeRipple(): () => void {
  let current:
    | {
        target: HTMLElement;
        layer: HTMLSpanElement;
        pointerId?: number;
        key?: string;
        startedAt: number;
        ending: boolean;
      }
    | undefined;
  let fadeTimer: ReturnType<typeof setTimeout> | undefined;
  let removeTimer: ReturnType<typeof setTimeout> | undefined;

  const clear = () => {
    clearTimeout(fadeTimer);
    clearTimeout(removeTimer);
    current?.layer.remove();
    current = undefined;
  };

  const finish = (cancelled = false) => {
    if (!current || current.ending) return;
    current.ending = true;
    const layer = current.layer;
    const delay = cancelled ? 0 : Math.max(0, MINIMUM_PRESS_MS - (performance.now() - current.startedAt));
    fadeTimer = setTimeout(() => {
      layer.classList.add('ripple-releasing');
      removeTimer = setTimeout(clear, FADE_MS);
    }, delay);
  };

  const findTarget = (event: Event) => {
    if (!(event.target instanceof Element)) return;
    const target = event.target.closest<HTMLElement>(TARGET_SELECTOR);
    if (!target || event.target.closest(':disabled, [aria-disabled="true"], [inert], [data-ripple="off"]')) return;
    return target;
  };

  const start = (target: HTMLElement, clientX?: number, clientY?: number) => {
    clear();
    const rect = target.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    // Convert viewport coordinates to the target's local padding box, including CSS scaling.
    const scaleX = target.offsetWidth / rect.width || 1;
    const scaleY = target.offsetHeight / rect.height || 1;
    const width = target.clientWidth || rect.width;
    const height = target.clientHeight || rect.height;
    const x = clientX === undefined ? width / 2 : (clientX - rect.left) * scaleX - target.clientLeft;
    const y = clientY === undefined ? height / 2 : (clientY - rect.top) * scaleY - target.clientTop;
    const radius = Math.hypot(Math.max(x, width - x), Math.max(y, height - y));
    const layer = document.createElement('span');
    layer.className = 'ripple-layer';
    layer.setAttribute('aria-hidden', 'true');
    const wave = document.createElement('span');
    wave.className = 'ripple-wave';
    wave.style.width = wave.style.height = `${radius * 2}px`;
    wave.style.left = `${x - radius}px`;
    wave.style.top = `${y - radius}px`;
    layer.append(wave);
    target.append(layer);
    current = { target, layer, startedAt: performance.now(), ending: false };
  };

  const onPointerDown = (event: PointerEvent) => {
    if (!event.isPrimary || event.button !== 0) return;
    const target = findTarget(event);
    if (!target) return;
    start(target, event.clientX, event.clientY);
    if (current) current.pointerId = event.pointerId;
  };
  const onPointerMove = (event: PointerEvent) => {
    if (!current || current.ending || current.pointerId !== event.pointerId) return;
    const rect = current.target.getBoundingClientRect();
    if (
      !current.target.isConnected ||
      event.clientX < rect.left ||
      event.clientX > rect.right ||
      event.clientY < rect.top ||
      event.clientY > rect.bottom
    ) {
      finish(true);
    }
  };
  const onPointerUp = (event: PointerEvent) => {
    if (current?.pointerId === event.pointerId) finish();
  };
  const onPointerCancel = (event: PointerEvent) => {
    if (current?.pointerId === event.pointerId) finish(true);
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.repeat || (event.key !== ' ' && event.key !== 'Enter')) return;
    const target = findTarget(event);
    // Do not react to typing in an input nested inside a ripple surface.
    if (!target || event.target !== target) return;
    start(target);
    if (current) current.key = event.key;
  };
  const onKeyUp = (event: KeyboardEvent) => {
    if (current?.key === event.key) finish();
  };
  const onCancel = () => finish(true);
  const onFocusOut = (event: FocusEvent) => {
    if (current && event.target instanceof Node && current.target.contains(event.target)) finish(true);
  };
  const onScroll = (event: Event) => {
    if (current && (event.target === document || (event.target instanceof Element && event.target.contains(current.target)))) finish(true);
  };
  const onVisibilityChange = () => {
    if (document.hidden) clear();
  };

  // Capture keeps feedback independent of child handlers that stop propagation.
  // Passive listeners leave native scrolling and click behavior untouched.
  const controller = new AbortController();
  const options = { capture: true, passive: true, signal: controller.signal };
  document.addEventListener('pointerdown', onPointerDown, options);
  document.addEventListener('pointermove', onPointerMove, options);
  document.addEventListener('pointerup', onPointerUp, options);
  document.addEventListener('pointercancel', onPointerCancel, options);
  document.addEventListener('keydown', onKeyDown, options);
  document.addEventListener('keyup', onKeyUp, options);
  document.addEventListener('contextmenu', onCancel, options);
  document.addEventListener('scroll', onScroll, options);
  document.addEventListener('focusout', onFocusOut, options);
  document.addEventListener(CANCEL_EVENT, onCancel, options);
  document.addEventListener('visibilitychange', onVisibilityChange, options);
  window.addEventListener('blur', clear, options);

  return () => {
    controller.abort();
    clear();
  };
}
