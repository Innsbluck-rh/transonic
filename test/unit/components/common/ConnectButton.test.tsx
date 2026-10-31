import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import ConnectButton from '~/components/common/ConnectButton';
import { device, resetConnectStore, setConnectAvailable, sharedPlayback } from '~test/fixtures/connect';

describe('ConnectButton', () => {
  let container: HTMLDivElement;
  let dispose: (() => void) | undefined;

  beforeEach(() => {
    resetConnectStore();
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    resetConnectStore();
  });

  it('ignores outside clicks before the menu has been opened', () => {
    dispose = render(() => <ConnectButton />, container);

    // dispatchEvent never rethrows listener errors; Vitest reports them as unhandled errors and fails the run.
    document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(container.textContent).not.toContain("there's no devices");
  });

  it('opens and closes safely from the trigger button', () => {
    dispose = render(() => <ConnectButton />, container);

    const button = container.querySelector('button');
    expect(button).not.toBeNull();

    button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(container.textContent).toContain("there's no devices");

    button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(container.textContent).not.toContain("there's no devices");
  });

  it('closes safely from an outside click after the menu opens', () => {
    dispose = render(() => <ConnectButton />, container);

    const button = container.querySelector('button');
    expect(button).not.toBeNull();

    button!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(container.textContent).toContain("there's no devices");

    document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(container.textContent).not.toContain("there's no devices");
  });

  it('uses the volume icon for the active playing device', () => {
    setConnectAvailable([device('remote-device', 'Living Room')], sharedPlayback({ activeDeviceId: 'remote-device', playingState: 'playing' }));

    dispose = render(() => <ConnectButton />, container);

    const icon = container.querySelector('i[class*="icon-["]') as HTMLElement | null;
    expect(icon?.classList.contains('icon-[material-symbols--volume-up]')).toBe(true);
    expect(container.textContent).toContain('Living Room');
  });

  it('shows the local active device as italic This Device', () => {
    setConnectAvailable([device('remote-device', 'Living Room')], sharedPlayback({ activeDeviceId: 'local-device', playingState: 'paused' }));

    dispose = render(() => <ConnectButton />, container);

    const label = container.querySelector('button p') as HTMLElement | null;
    expect(label?.textContent).toBe('This Device');
    expect(label?.classList.contains('italic')).toBe(true);
  });

  it('shows an italic secondary fallback when nothing is active', () => {
    setConnectAvailable([device('remote-device', 'Living Room')], sharedPlayback({ activeDeviceId: null, playingState: 'idle' }));

    dispose = render(() => <ConnectButton />, container);

    const label = container.querySelector('button p') as HTMLElement | null;
    expect(label?.textContent).toBe('(Nothing Active)');
    expect(label?.classList.contains('italic')).toBe(true);
    expect(label?.classList.contains('text-secondary-text')).toBe(true);
  });
});
