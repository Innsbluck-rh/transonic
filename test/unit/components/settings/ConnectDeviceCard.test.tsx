import { render } from 'solid-js/web';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ConnectDeviceCard from '~/components/settings/ConnectDeviceCard';
import { device, setConnectAvailable, sharedPlayback } from '~test/fixtures/connect';

vi.mock('~/bindings', () => ({
  commands: {
    connectTransferPlayback: vi.fn().mockResolvedValue({ status: 'ok', data: null }),
  },
}));

describe('ConnectDeviceCard', () => {
  let container: HTMLDivElement;
  let dispose: () => void;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    dispose?.();
    container.remove();
    vi.clearAllMocks();
  });

  it('highlights the active device even when playback is paused', () => {
    setConnectAvailable([device('active-device', 'Living Room')], sharedPlayback({ activeDeviceId: 'active-device', playingState: 'paused' }));

    dispose = render(() => <ConnectDeviceCard device={device('active-device', 'Living Room')} />, container);

    const icon = container.querySelector('i[class*="icon-["]') as HTMLElement | null;
    const label = container.querySelector('p') as HTMLElement | null;
    expect(icon?.classList.contains('icon-[material-symbols--devices]')).toBe(true);
    expect(icon?.classList.contains('text-accent')).toBe(true);
    expect(label?.classList.contains('text-accent')).toBe(true);
  });

  it('uses the volume icon only for the active playing device', () => {
    setConnectAvailable([device('active-device', 'Living Room')], sharedPlayback({ activeDeviceId: 'active-device', playingState: 'playing' }));

    dispose = render(() => <ConnectDeviceCard device={device('active-device', 'Living Room')} />, container);

    const icon = container.querySelector('i[class*="icon-["]') as HTMLElement | null;
    expect(icon?.classList.contains('icon-[material-symbols--volume-up]')).toBe(true);
    expect(icon?.classList.contains('text-accent')).toBe(true);
  });
});
