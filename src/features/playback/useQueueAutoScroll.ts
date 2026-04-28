import { Accessor, createEffect } from 'solid-js';
import { type SongResponse } from '~/bindings';

interface QueueAutoScrollOptions {
  currentIndex: Accessor<number | null>;
  currentEntryId: Accessor<string | null>;
  queueKey?: Accessor<string>;
  defaultBehavior?: ScrollBehavior;
  initialBehavior?: ScrollBehavior;
  queueChangeBehavior?: ScrollBehavior;
}

function resolveItemScrollTop(container: HTMLElement, item: HTMLElement) {
  const containerRect = container.getBoundingClientRect();
  const itemRect = item.getBoundingClientRect();
  return itemRect.top - containerRect.top + container.scrollTop;
}

function resolveScrollBehavior(hasScrolled: boolean, hasQueueChanged: boolean, options: QueueAutoScrollOptions): ScrollBehavior {
  if (!hasScrolled) {
    return options.initialBehavior ?? options.defaultBehavior ?? 'smooth';
  }

  if (hasQueueChanged) {
    return options.queueChangeBehavior ?? options.defaultBehavior ?? 'smooth';
  }

  return options.defaultBehavior ?? 'smooth';
}

export function buildQueueAutoScrollKey(queue: SongResponse[]) {
  return queue.map((song) => song.id).join('\u0000');
}

export function useQueueAutoScroll(options: QueueAutoScrollOptions) {
  let scrollContainerRef: HTMLDivElement | undefined;
  const itemRefs: Array<HTMLDivElement | undefined> = [];
  let hasScrolled = false;
  let previousQueueKey: string | undefined;

  const setScrollContainerRef = (element: HTMLDivElement) => {
    scrollContainerRef = element;
  };

  const setItemRef = (index: number, element: HTMLDivElement | undefined) => {
    itemRefs[index] = element;
  };

  createEffect(() => {
    const index = options.currentIndex();
    const currentEntryId = options.currentEntryId();
    const queueKey = options.queueKey?.();
    if (index === null || currentEntryId === null) {
      previousQueueKey = queueKey;
      return;
    }

    const container = scrollContainerRef;
    const item = itemRefs[index];
    if (!container || !item) {
      return;
    }

    const hasQueueChanged = queueKey !== undefined && queueKey !== previousQueueKey;
    const behavior = resolveScrollBehavior(hasScrolled, hasQueueChanged, options);

    container.scrollTo({
      top: Math.max(0, resolveItemScrollTop(container, item)),
      behavior,
    });

    hasScrolled = true;
    previousQueueKey = queueKey;
  });

  return {
    setScrollContainerRef,
    setItemRef,
  };
}
