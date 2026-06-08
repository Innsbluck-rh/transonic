// @refresh reload

import { MetaProvider } from '@solidjs/meta';
import { Navigate, Route, Router } from '@solidjs/router';
import PlaybackErrorDialog from './components/common/dialog/PlaybackErrorDialog';
import PlaybackStreamInfoDialog from './components/common/dialog/PlaybackStreamInfoDialog';
import PCContextMenu from './components/menu/PCContextMenu';
import SPBottomContextMenu from './components/menu/SPBottomContextMenu';
import AppLayout from './layouts/AppLayout';
import DesktopHomeLayout from './layouts/DesktopHomeLayout';
import SPHomeLayout from './layouts/SPHomeLayout';
import BrowseAlbum from './routes/browse/album';
import BrowseArtist from './routes/browse/artist';
import BrowseFolderStructure from './routes/browse/folder_structure';
import AppError from './routes/error';
import Home from './routes/home';
import InitLoad from './routes/init_load';
import InitLogin from './routes/init_login';
import Search from './routes/search';
import SettingsRoute from './routes/settings';
import SPBrowseStack from './routes/sp/browseStack';
import SPHome from './routes/sp/home';
import { isSP } from './utils/isSP';

function RootRedirect() {
  return <Navigate href={isSP() ? '/sp' : '/home'} />;
}

export default function App() {
  return (
    <Router
      root={(props) => (
        <MetaProvider>
          <title>transonic</title>
          <div>
            <main>{props.children}</main>
            <PCContextMenu />
            <SPBottomContextMenu />
            <PlaybackErrorDialog />
            <PlaybackStreamInfoDialog />
          </div>
        </MetaProvider>
      )}
    >
      <Route component={AppLayout}>
        <Route path='/' component={RootRedirect} />
        <Route component={InitLoad}>
          <Route path='/home/error' component={AppError} />
          <Route component={DesktopHomeLayout}>
            <Route path='/home' component={Home} />
            <Route path='/settings' component={SettingsRoute} />
            <Route path='/browse/folders/:libraryId' component={BrowseFolderStructure} />
            <Route path='/browse/folders/:libraryId/:nodeId' component={BrowseFolderStructure} />
            <Route path='/browse/artists/:id' component={BrowseArtist} />
            <Route path='/browse/album/:id' component={BrowseAlbum} />
            <Route path='/search' component={Search} />
          </Route>
          <Route component={SPHomeLayout}>
            <Route path='/sp' component={SPHome} />
            <Route path='/sp/browse' component={SPBrowseStack}>
              <Route path='/' />
              <Route path='/artists/:id' component={BrowseArtist} />
              <Route path='/albums/:id' component={BrowseAlbum} />
              <Route path='/folders/:libraryId' component={BrowseFolderStructure} />
              <Route path='/folders/:libraryId/:nodeId' component={BrowseFolderStructure} />
            </Route>
            <Route path='/sp/settings' component={SettingsRoute} />
            <Route path='/sp/search' component={Search} />
          </Route>
        </Route>
        <Route path='/init_login' component={InitLogin} />
        <Route path='*404' component={AppError} />
      </Route>
    </Router>
  );
}
