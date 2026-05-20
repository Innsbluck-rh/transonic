// @refresh reload

import { MetaProvider } from '@solidjs/meta';
import { Navigate, Route, Router } from '@solidjs/router';
import PlaybackErrorDialog from './components/common/dialog/PlaybackErrorDialog';
import PCContextMenu from './components/menu/PCContextMenu';
import SPBottomContextMenu from './components/menu/SPBottomContextMenu';
import AppLayout from './layouts/AppLayout';
import DesktopHomeLayout from './layouts/DesktopHomeLayout';
import MobileHomeLayout from './layouts/MobileHomeLayout';
import BrowseAlbum from './routes/browse/album';
import BrowseArtist from './routes/browse/artist';
import BrowseFolderStructure from './routes/browse/folder_structure';
import AppError from './routes/error';
import Home from './routes/home';
import InitLoad from './routes/init_load';
import InitLogin from './routes/init_login';
import SettingsRoute from './routes/settings';
import SPBrowseIndex from './routes/sp/browse';
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
          </Route>
          <Route component={MobileHomeLayout}>
            <Route path='/sp' component={SPHome} />
            <Route path='/sp/browse' component={SPBrowseIndex} />
            <Route path='/sp/settings' component={SettingsRoute} />
            <Route path='/sp/artists/:id' component={BrowseArtist} />
            <Route path='/sp/folders/:libraryId' component={BrowseFolderStructure} />
            <Route path='/sp/folders/:libraryId/:nodeId' component={BrowseFolderStructure} />
            <Route path='/sp/albums/:id' component={BrowseAlbum} />
          </Route>
        </Route>
        <Route path='/init_login' component={InitLogin} />
        <Route path='*404' component={AppError} />
      </Route>
    </Router>
  );
}
