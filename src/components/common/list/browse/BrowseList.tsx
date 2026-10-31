import { useLocation } from '@solidjs/router';
import { Component, createEffect, createSignal, For, Match, onCleanup, Show, Switch } from 'solid-js';
import { CoverArtSizes } from '~/features/albums/CoverArtSizes';
import { useCoverArtMap } from '~/features/albums/useCoverArtMap';
import type { BrowseListItem } from '~/features/browse/service';
import { useSPNavigate } from '~/features/navigation/useSPNavigate';
import ArtistRow from './row/ArtistRow';
import FolderRow from './row/FolderRow';

interface BrowseListProps {
  items: BrowseListItem[];
  emptyMessage: string;
  selectedId?: string | null;
  onClickItem?: (item: BrowseListItem) => void;
}

/**
 * @description Indexなどのための汎用リスト
 */
const BrowseList: Component<BrowseListProps> = (props) => {
  const location = useLocation();
  const navigate = useSPNavigate();
  const requestedCoverArtIdSet = new Set<string>();
  const [requestedCoverArtIds, setRequestedCoverArtIds] = createSignal<readonly string[]>([]);
  let coverArtObserver: IntersectionObserver | undefined;

  const requestCoverArtId = (coverArtId: string | null | undefined) => {
    const normalizedId = coverArtId?.trim();
    if (!normalizedId || requestedCoverArtIdSet.has(normalizedId)) {
      return;
    }

    requestedCoverArtIdSet.add(normalizedId);
    setRequestedCoverArtIds([...requestedCoverArtIdSet]);
  };

  const ensureCoverArtObserver = () => {
    if (typeof IntersectionObserver === 'undefined') {
      return undefined;
    }
    if (!coverArtObserver) {
      coverArtObserver = new IntersectionObserver((entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) {
            continue;
          }

          const coverArtId = (entry.target as HTMLElement).dataset.coverArtId;
          requestCoverArtId(coverArtId);
          coverArtObserver?.unobserve(entry.target);
        }
      });
    }

    return coverArtObserver;
  };

  const observeCoverArt = (coverArtId: string | null) => (element: HTMLDivElement) => {
    const normalizedId = coverArtId?.trim();
    if (!normalizedId) {
      return;
    }

    const observer = ensureCoverArtObserver();
    if (!observer) {
      requestCoverArtId(normalizedId);
      return;
    }

    element.dataset.coverArtId = normalizedId;
    observer.observe(element);
  };

  createEffect<string | undefined>((previousItemsKey) => {
    const nextItemsKey = props.items.map((item) => item.id).join('\0');
    if (previousItemsKey !== undefined && previousItemsKey !== nextItemsKey) {
      requestedCoverArtIdSet.clear();
      setRequestedCoverArtIds([]);
    }

    return nextItemsKey;
  }, undefined);

  onCleanup(() => {
    coverArtObserver?.disconnect();
  });

  const coverArtMap = useCoverArtMap(requestedCoverArtIds, CoverArtSizes.sm, {
    cachedFallbackSizes: [CoverArtSizes.lg, CoverArtSizes.md],
  });

  return (
    <div class='flex flex-col pb-3'>
      <Show
        when={props.items.length > 0}
        fallback={
          <Show when={props.emptyMessage}>
            <p class='px-3 py-2 text-xs text-zinc-500'>{props.emptyMessage}</p>
          </Show>
        }
      >
        <For each={props.items}>
          {(item) => {
            const active = () => (item.href ? location.pathname.startsWith(item.href) : false);
            const selected = () => props.selectedId === item.id;
            const onClick = () => {
              props.onClickItem?.(item);
              if (item.href) navigate(item.href);
            };

            return (
              <>
                <Switch>
                  <Match when={item.type === 'artist' && item}>
                    {(artist) => (
                      <ArtistRow
                        ref={observeCoverArt(artist().coverArtId)}
                        label={artist().name}
                        coverArtUrl={coverArtMap.src(artist().coverArtId)}
                        active={active()}
                        selected={selected()}
                        onClick={onClick}
                      />
                    )}
                  </Match>
                  <Match when={item.type === 'folder' && item}>
                    {(folder) => <FolderRow label={folder().name} active={active()} selected={selected()} onClick={onClick} />}
                  </Match>
                </Switch>
                <div class='bg-secondary-border h-px w-full'></div>
              </>
            );
          }}
        </For>
      </Show>
    </div>
  );
};

export default BrowseList;
