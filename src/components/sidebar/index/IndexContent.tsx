import { Component, createSignal, Match, Switch } from 'solid-js';
import ArtistList from './list/ArtistList';
import FolderList from './list/FolderList';
import IndexModeSelect from './select/IndexModeSelect';

export type BrowseMode = 'Folder Structures' | 'Artists';

export const BROWSE_MODE_URLS: Record<BrowseMode, string> = {
  'Folder Structures': '/browse/folders',
  Artists: '/browse/artists',
};

const IndexContent: Component = () => {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Folder Structures');

  return (
    <div class='flex w-full flex-col overflow-y-hidden'>
      <IndexModeSelect
        defaultMode={browseMode()}
        onSelect={(mode) => {
          setBrowseMode(mode);
        }}
      />

      <div class='flex-1 overflow-y-auto'>
        <Switch>
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

export default IndexContent;
