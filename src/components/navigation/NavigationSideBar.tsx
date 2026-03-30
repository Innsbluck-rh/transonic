import { Select } from '@kobalte/core/select';
import { useLocation, useNavigate } from '@solidjs/router';
import { Component, createEffect, createMemo, createSignal, Show } from 'solid-js';
import type { ArtistIndexItem, FolderStructureRootNode, FolderStructureSource, MusicFolderSummary } from '~/bindings';
import { fetchFolderStructureRoots, setFolderStructureSelectedLibraryId } from '~/features/browse/folderStructureService';
import { fetchArtistIndexes, fetchMusicFolders } from '~/features/browse/service';
import { folderStructureStore } from '~/stores/browse/FolderStructureStore';
import { sessionStore } from '~/stores/session/SessionStore';
import BrowseList from '../list/BrowseList';

type BrowseMode = 'Folder Structure' | 'Artist' | 'Album Artist';

const BROWSE_MODES: BrowseMode[] = ['Folder Structure', 'Artist', 'Album Artist'];

type SidebarBrowseItem = ArtistIndexItem | FolderStructureRootNode;

function resolveBrowseMode(pathname: string): BrowseMode {
  if (pathname.startsWith('/browse/artists')) {
    return 'Artist';
  }

  if (pathname.startsWith('/browse/album-artists')) {
    return 'Album Artist';
  }

  return 'Folder Structure';
}

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

  const viewMode = createMemo(() => resolveBrowseMode(location.pathname));
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
    if (viewMode() === 'Artist') {
      return artistRouteId();
    }

    if (viewMode() === 'Folder Structure') {
      return folderRoute().nodeId;
    }

    return null;
  });
  const showLibrarySelector = createMemo(() => viewMode() === 'Folder Structure' && musicFolders().length > 1);
  const emptyMessage = createMemo(() => {
    switch (viewMode()) {
      case 'Artist':
        return 'No artists available.';
      case 'Album Artist':
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
      case 'Artist':
        navigate('/browse/artists');
        break;
      case 'Album Artist':
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

    switch (viewMode()) {
      case 'Artist':
        void loadArtistItems();
        return;
      case 'Album Artist':
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
      <Select
        class='m-2'
        options={BROWSE_MODES}
        value={viewMode()}
        onChange={(mode) => {
          if (mode) {
            navigateToMode(mode);
          }
        }}
        itemComponent={(props) => {
          if (props.item.rawValue === viewMode()) return <></>;
          return (
            <Select.Item item={props.item} class='p-0.5 text-sm select-none cursor-pointer hover:bg-zinc-200'>
              <Select.ItemLabel>{props.item.rawValue}</Select.ItemLabel>
            </Select.Item>
          );
        }}
      >
        <Select.Trigger class='w-full text-start line-clamp-1'>
          <Select.Value<string>>{(state) => state.selectedOption()}</Select.Value>
        </Select.Trigger>
        <Select.Portal>
          <Select.Content class='bg-zinc-50 border border-zinc-800 rounded-md m-0 p-0'>
            <Select.Listbox class='m-0 p-0' />
          </Select.Content>
        </Select.Portal>
      </Select>

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

      <Show when={viewMode() === 'Folder Structure' && folderStructureSource() === 'indexes'}>
        <p class='px-3 pb-2 text-xs text-zinc-500'>This server provides a folder-like browse tree.</p>
      </Show>

      <div class='flex-1 overflow-y-auto'>
        <BrowseList
          items={items()}
          isLoading={isLoading()}
          error={error()}
          emptyMessage={emptyMessage()}
          selectedId={selectedItemId()}
          onClickItem={(item) => {
            if (viewMode() === 'Artist') {
              navigate(`/browse/artists/${encodeURIComponent(item.id)}`);
              return;
            }

            const libraryId = effectiveFolderLibraryId();
            if (viewMode() === 'Folder Structure' && libraryId) {
              navigate(`/browse/folders/${encodeURIComponent(libraryId)}/${encodeURIComponent(item.id)}`);
            }
          }}
        />
      </div>
    </div>
  );
};

export default NavigationSideBar;
