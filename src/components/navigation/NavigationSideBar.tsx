import { Select } from '@kobalte/core/select';
import { useLocation, useNavigate } from '@solidjs/router';
import { Component, createEffect, createMemo, createSignal } from 'solid-js';
import { fetchArtistIndexes, fetchMusicFolders } from '~/features/browse/service';
import { ArtistIndexItem, MusicFolderSummary } from '~/models/browse';
import { sessionStore } from '~/stores/session/SessionStore';
import BrowseList from '../list/BrowseList';

type BrowseMode = 'Folder Structure' | 'Artist' | 'Album Artist';

const BROWSE_MODES: BrowseMode[] = ['Folder Structure', 'Artist', 'Album Artist'];

type SidebarBrowseItem = MusicFolderSummary | ArtistIndexItem;

function resolveBrowseMode(pathname: string): BrowseMode {
  if (pathname.startsWith('/browse/artists')) {
    return 'Artist';
  }

  if (pathname.startsWith('/browse/album-artists')) {
    return 'Album Artist';
  }

  return 'Folder Structure';
}

const NavigationSideBar: Component = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const [items, setItems] = createSignal<SidebarBrowseItem[]>([]);
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const viewMode = createMemo(() => resolveBrowseMode(location.pathname));
  const emptyMessage = createMemo(() => {
    switch (viewMode()) {
      case 'Artist':
        return 'No artists available.';
      case 'Album Artist':
        return 'Album artist browse is not implemented yet.';
      default:
        return 'No music folders available.';
    }
  });

  async function loadBrowseItems(mode: BrowseMode) {
    if (!sessionStore.activeSession) {
      setItems([]);
      setError(null);
      setIsLoading(false);
      return;
    }

    if (mode === 'Album Artist') {
      setItems([]);
      setError(null);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      if (mode === 'Artist') {
        const response = await fetchArtistIndexes();
        setItems(response.artists);
      } else {
        const response = await fetchMusicFolders();
        setItems(response.musicFolders);
      }
    } catch (invokeError) {
      console.error(invokeError);
      setItems([]);
      setError(`Failed to load ${mode === 'Artist' ? 'artists' : 'music folders'}.`);
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
      default:
        navigate('/browse/folders');
        break;
    }
  }

  createEffect(() => {
    void sessionStore.activeSession;
    void loadBrowseItems(viewMode());
  });

  return (
    <div class='flex flex-col min-w-36 w-46 border-r border-zinc-600 resize-x overflow-auto'>
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
          {/* <Select.Icon class="select__icon">
          <CaretSortIcon />
        </Select.Icon> */}
        </Select.Trigger>
        <Select.Portal>
          <Select.Content class='bg-zinc-50 border border-zinc-800 rounded-md m-0 p-0'>
            <Select.Listbox class='m-0 p-0' />
          </Select.Content>
        </Select.Portal>
      </Select>

      <div class='flex-1 overflow-y-auto'>
        <BrowseList
          items={items()}
          isLoading={isLoading()}
          error={error()}
          emptyMessage={emptyMessage()}
          onClickItem={(item) => {
            if (viewMode() === 'Artist') {
              navigate(`/browse/artists/${encodeURIComponent(item.id)}`);
              return;
            }

            if (viewMode() === 'Folder Structure') {
              navigate(`/browse/folders/${encodeURIComponent(item.id)}`);
            }
          }}
        />
      </div>
    </div>
  );
};

export default NavigationSideBar;
