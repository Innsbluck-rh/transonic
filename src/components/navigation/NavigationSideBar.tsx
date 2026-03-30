import { useLocation, useNavigate } from '@solidjs/router';
import { Component, createEffect, createMemo, createSignal, Show } from 'solid-js';
import type { ArtistIndexItem, FolderStructureRootNode, FolderStructureSource, MusicFolderSummary } from '~/bindings';
import { fetchFolderStructureRoots, setFolderStructureSelectedLibraryId } from '~/features/browse/folderStructureService';
import { fetchArtistIndexes, fetchMusicFolders } from '~/features/browse/service';
import { folderStructureStore } from '~/stores/browse/FolderStructureStore';
import { sessionStore } from '~/stores/session/SessionStore';
import BrowseList from '../list/BrowseList';
import NavigationTypeSelect from './select/NavigationTypeSelect';

export type BrowseMode = 'Folder Structures' | 'Artists' | 'Album Artists';

export const BROWSE_MODE_URLS: Record<BrowseMode, string> = {
  'Folder Structures': '/browse/folders',
  Artists: '/browse/artists',
  'Album Artists': '/browse/album-artists',
};

export function resolveBrowseMode(pathname: string): BrowseMode {
  const foundIndex = Object.values(BROWSE_MODE_URLS).findIndex((url) => pathname.startsWith(url));
  if (foundIndex < 0) return 'Folder Structures';

  const found = Object.keys(BROWSE_MODE_URLS)[foundIndex] as BrowseMode;
  return found;
}

type SidebarBrowseItem = ArtistIndexItem | FolderStructureRootNode;

function decodePathParam(value?: string) {
  if (!value) {
    return null;
  }

  try {
    const trimmed = decodeURIComponent(value).trim();
    return trimmed.length > 0 ? trimmed : null;
  } catch (_error) {
    return value;
  }
}

function resolveFolderStructureRoute(pathname: string) {
  const segments = pathname.split('/').filter(Boolean);
  if (segments[0] !== 'browse' || segments[1] !== 'folders') {
    return {
      libraryId: null,
      nodeId: null,
    };
  }

  return {
    libraryId: decodePathParam(segments[2]),
    nodeId: decodePathParam(segments[3]),
  };
}

function resolveArtistRouteId(pathname: string) {
  const segments = pathname.split('/').filter(Boolean);
  if (segments[0] !== 'browse' || segments[1] !== 'artists') {
    return null;
  }

  return decodePathParam(segments[2]);
}

function findLibrary(musicFolders: MusicFolderSummary[], libraryId: string | null) {
  if (!libraryId) {
    return null;
  }

  return musicFolders.find((folder) => folder.id === libraryId) ?? null;
}

const NavigationSideBar: Component = () => {
  const navigate = useNavigate();
  const location = useLocation();

  const [items, setItems] = createSignal<SidebarBrowseItem[]>([]);
  const [musicFolders, setMusicFolders] = createSignal<MusicFolderSummary[]>([]);
  const [folderStructureSource, setFolderStructureSource] = createSignal<FolderStructureSource | null>(null);
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const [browseMode, setBrowseMode] = createSignal<BrowseMode>(resolveBrowseMode(location.pathname));
  const folderRoute = createMemo(() => resolveFolderStructureRoute(location.pathname));
  const artistRouteId = createMemo(() => resolveArtistRouteId(location.pathname));
  const effectiveFolderLibraryId = createMemo(() => {
    const matchedRouteLibrary = findLibrary(musicFolders(), folderRoute().libraryId);
    if (matchedRouteLibrary) {
      return matchedRouteLibrary.id;
    }

    const matchedStoredLibrary = findLibrary(musicFolders(), folderStructureStore.selectedLibraryId);
    if (matchedStoredLibrary) {
      return matchedStoredLibrary.id;
    }

    if (musicFolders().length === 1) {
      return musicFolders()[0].id;
    }

    return null;
  });
  const selectedItemId = createMemo(() => {
    if (browseMode() === 'Artists') {
      return artistRouteId();
    }

    if (browseMode() === 'Folder Structures') {
      return folderRoute().nodeId;
    }

    return null;
  });
  const showLibrarySelector = createMemo(() => browseMode() === 'Folder Structures' && musicFolders().length > 1);
  const emptyMessage = createMemo(() => {
    switch (browseMode()) {
      case 'Artists':
        return 'No artists available.';
      case 'Album Artists':
        return 'Album artist browse is not implemented yet.';
      default:
        if (musicFolders().length === 0 && !isLoading()) {
          return 'No libraries available.';
        }

        if (!effectiveFolderLibraryId()) {
          return 'Select a library.';
        }

        return 'No top-level folders available.';
    }
  });

  async function loadArtistItems() {
    setMusicFolders([]);
    setFolderStructureSource(null);
    setIsLoading(true);
    setError(null);

    try {
      const response = await fetchArtistIndexes();
      setItems(response.artists);
    } catch (invokeError) {
      console.error(invokeError);
      setItems([]);
      setError('Failed to load artists.');
    } finally {
      setIsLoading(false);
    }
  }

  async function loadFolderStructureItems() {
    setIsLoading(true);
    setError(null);

    try {
      const musicFoldersResponse = await fetchMusicFolders();
      setMusicFolders(musicFoldersResponse.musicFolders);

      const selectedLibraryId =
        findLibrary(musicFoldersResponse.musicFolders, folderRoute().libraryId)?.id ??
        findLibrary(musicFoldersResponse.musicFolders, folderStructureStore.selectedLibraryId)?.id ??
        (musicFoldersResponse.musicFolders.length === 1 ? musicFoldersResponse.musicFolders[0].id : null);

      setFolderStructureSelectedLibraryId(selectedLibraryId);

      if (!selectedLibraryId) {
        setItems([]);
        setFolderStructureSource(null);
        return;
      }

      const response = await fetchFolderStructureRoots({ libraryId: selectedLibraryId });
      setItems(response.rootNodes);
      setFolderStructureSource(response.source);
    } catch (invokeError) {
      console.error(invokeError);
      setItems([]);
      setFolderStructureSource(null);
      setError('Failed to load folder structure.');
    } finally {
      setIsLoading(false);
    }
  }

  function navigateToMode(mode: BrowseMode) {
    switch (mode) {
      case 'Artists':
        navigate('/browse/artists');
        break;
      case 'Album Artists':
        navigate('/browse/album-artists');
        break;
      default: {
        const libraryId = folderStructureStore.selectedLibraryId ?? effectiveFolderLibraryId();
        if (libraryId) {
          navigate(`/browse/folders/${encodeURIComponent(libraryId)}`);
        } else {
          navigate('/browse/folders');
        }
        break;
      }
    }
  }

  createEffect(() => {
    void sessionStore.activeSession;
    void location.pathname;

    if (!sessionStore.activeSession) {
      setItems([]);
      setMusicFolders([]);
      setFolderStructureSource(null);
      setError(null);
      setIsLoading(false);
      return;
    }

    switch (browseMode()) {
      case 'Artists':
        void loadArtistItems();
        return;
      case 'Album Artists':
        setItems([]);
        setMusicFolders([]);
        setFolderStructureSource(null);
        setError(null);
        setIsLoading(false);
        return;
      default:
        void loadFolderStructureItems();
        return;
    }
  });

  return (
    <div class='flex flex-col min-w-40 w-56 border-r border-zinc-600 resize-x overflow-auto'>
      <NavigationTypeSelect
        defaultMode={browseMode()}
        onSelect={(mode) => {
          setBrowseMode(mode);
          navigateToMode(mode);
        }}
      />

      <Show when={showLibrarySelector()}>
        <div class='px-2 pb-2'>
          <select
            class='w-full rounded border border-zinc-300 bg-white px-2 py-1 text-xs text-zinc-700'
            value={effectiveFolderLibraryId() ?? ''}
            onChange={(event) => {
              const nextLibraryId = event.currentTarget.value || null;
              setFolderStructureSelectedLibraryId(nextLibraryId);

              if (nextLibraryId) {
                navigate(`/browse/folders/${encodeURIComponent(nextLibraryId)}`);
                return;
              }

              navigate('/browse/folders');
            }}
          >
            <option value=''>Select library</option>
            {musicFolders().map((folder) => (
              <option value={folder.id}>{folder.name}</option>
            ))}
          </select>
        </div>
      </Show>

      <div class='flex-1 overflow-y-auto'>
        <BrowseList
          items={items()}
          isLoading={isLoading()}
          error={error()}
          emptyMessage={emptyMessage()}
          selectedId={selectedItemId()}
          onClickItem={(item) => {
            if (browseMode() === 'Artists') {
              navigate(`/browse/artists/${encodeURIComponent(item.id)}`);
              return;
            }

            const libraryId = effectiveFolderLibraryId();
            if (browseMode() === 'Folder Structures' && libraryId) {
              navigate(`/browse/folders/${encodeURIComponent(libraryId)}/${encodeURIComponent(item.id)}`);
            }
          }}
        />
      </div>
    </div>
  );
};

export default NavigationSideBar;
