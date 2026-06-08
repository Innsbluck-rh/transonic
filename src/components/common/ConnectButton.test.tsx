// @vitest-environment jsdom

import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ConnectButton from './ConnectButton';

vi.mock('@iconify-icon/solid', () => ({
  Icon: () => null,
}));

describe('ConnectButton', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    dispose = render(() => <ConnectButton />, container);
  });

  afterEach(() => {
    dispose();
    container.remove();
  });

  it('ignores outside clicks before the menu has been opened', () => {
    expect(() => {
      document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }).not.toThrow();
  });

  it('opens and closes safely from the trigger button', () => {
    const button = container.querySelector('button');
    expect(button).not.toBeNull();

    expect(() => {
      button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }).not.toThrow();
    expect(container.textContent).toContain("there's no devices");

    expect(() => {
      button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }).not.toThrow();
    expect(container.textContent).not.toContain("there's no devices");
  });

  it('closes safely from an outside click after the menu opens', () => {
    const button = container.querySelector('button');
    expect(button).not.toBeNull();

    button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(container.textContent).toContain("there's no devices");

    expect(() => {
      document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    }).not.toThrow();
    expect(container.textContent).not.toContain("there's no devices");
  });
});
