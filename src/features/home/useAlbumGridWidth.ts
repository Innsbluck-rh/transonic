import { Accessor, createSignal, onCleanup } from 'solid-js';

// Basically this is just an width observer but who cares
export function useAlbumGridWidth(): { albumGridWidth: Accessor<number>; observeRef: (el: HTMLElement) => void } {
  const [albumGridWidth, setAlbumGridWidth] = createSignal(0);

  const albumGridObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      setAlbumGridWidth(entry.contentRect.width);
    }
  });

  onCleanup(() => albumGridObserver.disconnect());

  const observeRef = (el: HTMLElement) => {
    albumGridObserver.observe(el);
    onCleanup(() => albumGridObserver.unobserve(el));
  };

  return { albumGridWidth, observeRef };
}
