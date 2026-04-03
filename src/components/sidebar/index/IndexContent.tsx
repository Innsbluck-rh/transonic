import { useNavigate } from '@solidjs/router';
import { Component, createMemo, createSignal, Match, Switch } from 'solid-js';
import { resolveBrowseIndexRoute, type RouteVariant } from '~/features/navigation/routes';
import ArtistList from './list/ArtistList';
import FolderList from './list/FolderList';
import IndexModeSelect from './select/IndexModeSelect';

export type BrowseMode = 'Folder Structures' | 'Artists';

export const BROWSE_MODE_URLS: Record<BrowseMode, string> = {
  'Folder Structures': '/browse/folders',
  Artists: '/browse/artists',
};

interface IndexContentProps {
  initialMode?: BrowseMode;
  modeNavigation?: 'local' | 'route';
  routeVariant?: RouteVariant;
}

const IndexContent: Component<IndexContentProps> = (props) => {
  const navigate = useNavigate();
  const [browseMode, setBrowseMode] = createSignal<BrowseMode>(props.initialMode ?? 'Artists');
  const routeVariant = createMemo<RouteVariant>(() => props.routeVariant ?? 'desktop');
  const selectedMode = createMemo<BrowseMode>(() => (props.modeNavigation === 'route' ? (props.initialMode ?? 'Artists') : browseMode()));

  return (
    <div class='flex w-full flex-col overflow-y-hidden'>
      <IndexModeSelect
        defaultMode={selectedMode()}
        onSelect={(mode) => {
          if (props.modeNavigation === 'route') {
            navigate(resolveBrowseIndexRoute(mode, routeVariant()));
            return;
          }

          setBrowseMode(mode);
        }}
      />

      <div class='flex-1 overflow-y-auto'>
        <Switch>
          <Match when={selectedMode() === 'Artists'}>
            <ArtistList routeVariant={routeVariant()} />
          </Match>
          <Match when={selectedMode() === 'Folder Structures'}>
            <FolderList routeVariant={routeVariant()} />
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default IndexContent;
