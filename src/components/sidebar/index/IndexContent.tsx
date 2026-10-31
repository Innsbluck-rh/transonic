import { Component, createSignal, Match, Switch } from 'solid-js';
import { type BrowseMode } from '~/features/navigation/routes';
import IndexModeSelect from './IndexModeSelect';
import ArtistList from './list/ArtistList';
import FolderList from './list/FolderList';

const IndexContent: Component = () => {
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>('Artists');

  return (
    <div class='relative flex size-full flex-col overflow-y-auto'>
      <div class='sticky top-0 z-1 box-border flex p-1.5'>
        <IndexModeSelect
          defaultMode={browseMode()}
          onSelect={(mode) => {
            setBrowseMode(mode);
          }}
        />
      </div>
      <div class='flex flex-col'>
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
