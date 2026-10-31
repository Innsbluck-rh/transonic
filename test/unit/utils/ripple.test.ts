import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cancelRipple, initializeRipple } from '~/utils/ripple';

function pointer(target: EventTarget, type: string, values: Partial<PointerEventInit> = {}) {
  const event = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: 40, clientY: 20, button: 0, ...values });
  Object.defineProperties(event, {
    pointerId: { value: values.pointerId ?? 1 },
    isPrimary: { value: values.isPrimary ?? true },
  });
  target.dispatchEvent(event);
  return event;
}

function row(tag = 'div') {
  const target = document.createElement(tag);
  target.className = 'ripple';
  vi.spyOn(target, 'getBoundingClientRect').mockReturnValue({ left: 0, top: 0, right: 100, bottom: 40, width: 100, height: 40 } as DOMRect);
  document.body.append(target);
  return target;
}

const layers = () => document.getElementsByClassName('ripple-layer');

describe('delegated ripple', () => {
  let dispose: () => void;
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
    dispose = initializeRipple();
  });
  afterEach(() => {
    dispose();
    document.body.replaceChildren();
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('starts on contact, preserves clicks, and gives quick taps a minimum visible duration', () => {
    const target = row();
    const click = vi.fn();
    target.addEventListener('click', click);
    expect(pointer(target, 'pointerdown').defaultPrevented).toBe(false);
    expect(layers()).toHaveLength(1);
    expect(click).not.toHaveBeenCalled();
    pointer(target, 'pointerup');
    target.click();
    expect(click).toHaveBeenCalledOnce();
    vi.advanceTimersByTime(179);
    expect(layers()[0].classList.contains('ripple-releasing')).toBe(false);
    vi.advanceTimersByTime(1);
    expect(layers()[0].classList.contains('ripple-releasing')).toBe(true);
    vi.advanceTimersByTime(160);
    expect(layers()).toHaveLength(0);
  });

  it('holds feedback until release without ongoing timers', () => {
    const target = row();
    pointer(target, 'pointerdown');
    expect(vi.getTimerCount()).toBe(0);
    vi.advanceTimersByTime(5000);
    expect(layers()).toHaveLength(1);
    pointer(document, 'pointerup');
    vi.advanceTimersByTime(160);
    expect(layers()).toHaveLength(0);
  });

  it.each(['pointercancel', 'outside', 'scroll', 'contextmenu', 'application'])('cancels a press on %s', (reason) => {
    const target = row();
    pointer(target, 'pointerdown');
    if (reason === 'outside') pointer(target, 'pointermove', { clientX: 101 });
    else if (reason === 'application') cancelRipple();
    else if (reason === 'scroll') document.dispatchEvent(new Event('scroll'));
    else pointer(target, reason);
    vi.advanceTimersByTime(160);
    expect(layers()).toHaveLength(0);
  });

  it('ignores another pointer and non-primary or secondary-button presses', () => {
    const target = row();
    pointer(target, 'pointerdown', { isPrimary: false });
    pointer(target, 'pointerdown', { button: 2 });
    expect(layers()).toHaveLength(0);
    pointer(target, 'pointerdown');
    pointer(target, 'pointercancel', { pointerId: 2 });
    vi.advanceTimersByTime(1000);
    expect(layers()).toHaveLength(1);
  });

  it('respects native and ARIA disabled controls, including nested descendants', () => {
    const button = row('button') as HTMLButtonElement;
    button.disabled = true;
    pointer(button, 'pointerdown');
    const target = row();
    target.setAttribute('aria-disabled', 'true');
    const child = document.createElement('i');
    target.append(child);
    pointer(child, 'pointerdown');
    expect(layers()).toHaveLength(0);
  });

  it('selects only the nearest target even when a child stops propagation', () => {
    const outer = row();
    const inner = row();
    outer.append(inner);
    const child = document.createElement('i');
    inner.append(child);
    child.addEventListener('pointerdown', (event) => event.stopPropagation());
    pointer(child, 'pointerdown');
    expect(layers()).toHaveLength(1);
    expect(layers()[0].parentElement).toBe(inner);
  });

  it('supports keyboard feedback on ordinary buttons without a ripple class', () => {
    const button = row('button');
    button.className = '';
    button.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));
    expect(layers()).toHaveLength(1);
    button.dispatchEvent(new KeyboardEvent('keyup', { key: ' ', bubbles: true }));
    vi.advanceTimersByTime(340);
    expect(layers()).toHaveLength(0);
  });

  it('bounds transient elements during repeated taps and cleans up detached targets', () => {
    const target = row();
    for (let i = 0; i < 20; i++) {
      pointer(target, 'pointerdown');
      pointer(target, 'pointerup');
    }
    expect(layers()).toHaveLength(1);
    target.remove();
    vi.advanceTimersByTime(340);
    expect(target.getElementsByClassName('ripple-layer')).toHaveLength(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('handles 2000 dynamically added rows with no per-row listeners or global selector scans', () => {
    const targets = Array.from({ length: 2000 }, () => row());
    const addListener = vi.spyOn(EventTarget.prototype, 'addEventListener');
    const queryAll = vi.spyOn(Document.prototype, 'querySelectorAll');
    const queryOne = vi.spyOn(Document.prototype, 'querySelector');
    const elementQueryAll = vi.spyOn(Element.prototype, 'querySelectorAll');
    expect(layers()).toHaveLength(0);
    pointer(targets[1999], 'pointerdown');
    expect(layers()).toHaveLength(1);
    expect(layers()[0].parentElement).toBe(targets[1999]);
    expect(addListener).not.toHaveBeenCalled();
    expect(queryAll).not.toHaveBeenCalled();
    expect(queryOne).not.toHaveBeenCalled();
    expect(elementQueryAll).not.toHaveBeenCalled();
    expect(targets[0].getBoundingClientRect).not.toHaveBeenCalled();
    expect(targets[1999].getBoundingClientRect).toHaveBeenCalledOnce();
  });

  it('does not cancel a new press when the previously focused control loses focus', () => {
    const previous = row('button');
    const target = row('button');
    pointer(target, 'pointerdown');
    previous.dispatchEvent(new FocusEvent('focusout', { bubbles: true, relatedTarget: target }));
    vi.advanceTimersByTime(1000);
    expect(layers()).toHaveLength(1);
    target.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
    vi.advanceTimersByTime(160);
    expect(layers()).toHaveLength(0);
  });
  it('disposes timers, decoration, and listeners and can be initialized again', () => {
    const target = row();
    pointer(target, 'pointerdown');
    pointer(target, 'pointerup');
    dispose();
    expect(layers()).toHaveLength(0);
    expect(vi.getTimerCount()).toBe(0);
    pointer(target, 'pointerdown');
    expect(layers()).toHaveLength(0);
    dispose = initializeRipple();
    pointer(target, 'pointerdown');
    expect(layers()).toHaveLength(1);
    window.dispatchEvent(new Event('blur'));
    expect(layers()).toHaveLength(0);
  });
});
