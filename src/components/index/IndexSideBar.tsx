import { Component, createSignal, Match, Switch } from 'solid-js';
import AlbumArtistList from './list/AlbumArtistList';
import ArtistList from './list/ArtistList';
import FolderList from './list/FolderList';
import IndexModeSelect from './select/IndexModeSelect';

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

const IndexSideBar: Component = () => {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Folder Structures');

  return (
    <div class='flex flex-col min-w-40 w-56 border-r border-zinc-600 resize-x overflow-auto'>
      <IndexModeSelect
        defaultMode={browseMode()}
        onSelect={(mode) => {
          setBrowseMode(mode);
        }}
      />

      <div class='flex-1 overflow-y-auto'>
        <Switch>
          <Match when={browseMode() === 'Album Artists'}>
            <AlbumArtistList />
          </Match>
          <Match when={browseMode() === 'Artists'}>
            <ArtistList />
          </Match>
          <Match when={browseMode() === 'Folder Structures'}>
            <FolderList />
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default IndexSideBar;
