// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import VolumeSlider from './VolumeSlider';

vi.mock('@iconify-icon/solid', () => ({
  Icon: (props: { icon: string; class?: string; onClick?: (event: MouseEvent) => void }) => (
    <span data-testid='volume-icon' data-icon={props.icon} class={props.class} onClick={props.onClick} />
  ),
}));

function pointerEvent(type: string, clientX: number) {
  const event = new MouseEvent(type, { bubbles: true, clientX });
  Object.defineProperty(event, 'pointerId', { value: 1 });
  return event;
}

function mockSliderRect(slider: Element) {
  vi.spyOn(slider, 'getBoundingClientRect').mockReturnValue({
    bottom: 20,
    height: 20,
    left: 0,
    right: 100,
    top: 0,
    width: 100,
    x: 0,
    y: 0,
    toJSON: () => {},
  });
}

function wheelEvent(deltaY: number) {
  return new WheelEvent('wheel', { bubbles: true, cancelable: true, deltaY });
}

describe('VolumeSlider', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    Object.defineProperty(HTMLElement.prototype, 'setPointerCapture', { configurable: true, value: vi.fn() });
    Object.defineProperty(HTMLElement.prototype, 'hasPointerCapture', { configurable: true, value: vi.fn(() => true) });
    Object.defineProperty(HTMLElement.prototype, 'releasePointerCapture', { configurable: true, value: vi.fn() });

    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    vi.restoreAllMocks();
  });

  it('keeps live volume changes while avoiding duplicate input on release', () => {
    const onVolumeInput = vi.fn();
    const onVolumeCommit = vi.fn();
    dispose = render(() => <VolumeSlider volume={0} onVolumeInput={onVolumeInput} onVolumeCommit={onVolumeCommit} />, container);

    const slider = container.querySelector('[role="slider"]');
    expect(slider).not.toBeNull();
    mockSliderRect(slider!);

    slider!.dispatchEvent(pointerEvent('pointerdown', 10));
    slider!.dispatchEvent(pointerEvent('pointermove', 40));
    slider!.dispatchEvent(pointerEvent('pointerup', 40));

    expect(onVolumeInput.mock.calls.map(([volume]) => volume)).toEqual([0.1, 0.4]);
    expect(onVolumeCommit.mock.calls.map(([volume]) => volume)).toEqual([0.4]);
  });

  it('sends the release position once when it changed after the last move', () => {
    const onVolumeInput = vi.fn();
    const onVolumeCommit = vi.fn();
    dispose = render(() => <VolumeSlider volume={0} onVolumeInput={onVolumeInput} onVolumeCommit={onVolumeCommit} />, container);

    const slider = container.querySelector('[role="slider"]');
    expect(slider).not.toBeNull();
    mockSliderRect(slider!);

    slider!.dispatchEvent(pointerEvent('pointerdown', 10));
    slider!.dispatchEvent(pointerEvent('pointermove', 40));
    slider!.dispatchEvent(pointerEvent('pointerup', 70));

    expect(onVolumeInput.mock.calls.map(([volume]) => volume)).toEqual([0.1, 0.4, 0.7]);
    expect(onVolumeCommit.mock.calls.map(([volume]) => volume)).toEqual([0.7]);
  });

  it('changes volume by one percentage point per wheel event', () => {
    const onVolumeInput = vi.fn();
    const onVolumeCommit = vi.fn();
    dispose = render(() => <VolumeSlider volume={0.5} onVolumeInput={onVolumeInput} onVolumeCommit={onVolumeCommit} />, container);

    const slider = container.querySelector('[role="slider"]');
    expect(slider).not.toBeNull();

    slider!.dispatchEvent(wheelEvent(-1));
    slider!.dispatchEvent(wheelEvent(1));

    expect(onVolumeInput.mock.calls.map(([volume]) => volume)).toEqual([0.51, 0.49]);
    expect(onVolumeCommit.mock.calls.map(([volume]) => volume)).toEqual([0.51, 0.49]);
  });

  it('ignores wheel events when disabled', () => {
    const onVolumeInput = vi.fn();
    const onVolumeCommit = vi.fn();
    dispose = render(() => <VolumeSlider volume={0.5} disabled onVolumeInput={onVolumeInput} onVolumeCommit={onVolumeCommit} />, container);

    const slider = container.querySelector('[role="slider"]');
    expect(slider).not.toBeNull();

    slider!.dispatchEvent(wheelEvent(-1));

    expect(onVolumeInput).not.toHaveBeenCalled();
    expect(onVolumeCommit).not.toHaveBeenCalled();
  });
});
