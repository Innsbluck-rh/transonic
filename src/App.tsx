// @refresh reload

import { MetaProvider } from '@solidjs/meta';
import { Navigate, Route, Router } from '@solidjs/router';
import PCContextMenu from './components/menu/PCContextMenu';
import SPBottomContextMenu from './components/menu/SPBottomContextMenu';
import { MOBILE_HOME_ROUTE, resolveHomeRoute } from './features/navigation/routes';
import BrowseAlbum from './routes/browse/album';
import BrowseArtist from './routes/browse/artist';
import BrowseFolderStructure from './routes/browse/folder_structure';
import AppError from './routes/error';
import Home from './routes/home';
import HomeLayout from './routes/homeLayout';
import HomeLayoutSP from './routes/homeLayoutSP';
import InitLoad from './routes/init_load';
import InitLogin from './routes/init_login';
import Layout from './routes/layout';
import SettingsRoute from './routes/settings';
import SPAlbum from './routes/sp/album';
import SPBrowseArtist from './routes/sp/artist';
import SPBrowseFolderStructure from './routes/sp/folder';
import SPIndexArtists from './routes/sp/index_artists';
import SPIndexFolders from './routes/sp/index_folders';

export default function App() {
  const homeRoute = resolveHomeRoute();

  return (
    <Router
      root={(props) => (
        <MetaProvider>
          <title>transonic</title>
          <div>
            <main>{props.children}</main>
          </div>
        </MetaProvider>
      )}
    >
      <Route component={Layout}>
        <Route path='/' component={() => <Navigate href={homeRoute} />} />
        <Route component={InitLoad}>
          <Route path='/home/error' component={AppError} />
          <Route component={HomeLayout}>
            <Route path='/home' component={Home} />
            <Route path='/settings' component={SettingsRoute} />
            <Route path='/browse/folders/:libraryId' component={BrowseFolderStructure} />
            <Route path='/browse/folders/:libraryId/:nodeId' component={BrowseFolderStructure} />
            <Route path='/browse/artists/:id' component={BrowseArtist} />
            <Route path='/browse/album/:id' component={BrowseAlbum} />
          </Route>
          <Route component={HomeLayoutSP}>
            <Route path='/sp' component={() => <Navigate href={MOBILE_HOME_ROUTE} />} />
            <Route path='/sp/index/artists' component={SPIndexArtists} />
            <Route path='/sp/index/folders' component={SPIndexFolders} />
            <Route path='/sp/setting' component={SettingsRoute} />
            <Route path='/sp/artists/:id' component={SPBrowseArtist} />
            <Route path='/sp/folders/:libraryId' component={SPBrowseFolderStructure} />
            <Route path='/sp/folders/:libraryId/:nodeId' component={SPBrowseFolderStructure} />
            <Route path='/sp/albums/:id' component={SPAlbum} />
          </Route>
        </Route>
        <Route path='/init_login' component={InitLogin} />
        <Route path='*404' component={AppError} />
      </Route>
      <PCContextMenu />
      <SPBottomContextMenu />
    </Router>
  );
}
