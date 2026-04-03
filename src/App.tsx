// @refresh reload

import { MetaProvider } from '@solidjs/meta';
import { Navigate, Route, Router } from '@solidjs/router';
import { platform } from '@tauri-apps/plugin-os';
import { MOBILE_HOME_ROUTE, resolveHomeRoute } from './features/navigation/routes';
import BrowseArtist from './routes/browse/artist';
import BrowseFolderStructure from './routes/browse/folder_structure';
import AppError from './routes/error';
import Home from './routes/home';
import HomeLayout from './routes/homeLayout';
import HomeLayoutSP from './routes/homeLayoutSP';
import InitLoad from './routes/init_load';
import InitLogin from './routes/init_login';
import Layout from './routes/layout';
import SPAlbum from './routes/sp/album';
import SPBrowseArtist from './routes/sp/artist';
import SPBrowseFolderStructure from './routes/sp/folder';
import SPIndexArtists from './routes/sp/index_artists';
import SPIndexFolders from './routes/sp/index_folders';
import SPPlayer from './routes/sp/player';

export default function App() {
  const pf = platform();
  const isSP = pf === 'android' || pf === 'ios';
  const homeRoute = resolveHomeRoute(isSP ? 'mobile' : 'desktop');

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
            <Route path='/browse/folders/:libraryId' component={BrowseFolderStructure} />
            <Route path='/browse/folders/:libraryId/:nodeId' component={BrowseFolderStructure} />
            <Route path='/browse/artists/:id' component={BrowseArtist} />
          </Route>
          <Route component={HomeLayoutSP}>
            <Route path='/sp' component={() => <Navigate href={MOBILE_HOME_ROUTE} />} />
            <Route path='/sp/index/artists' component={SPIndexArtists} />
            <Route path='/sp/index/folders' component={SPIndexFolders} />
            <Route path='/sp/artists/:id' component={SPBrowseArtist} />
            <Route path='/sp/folders/:libraryId' component={SPBrowseFolderStructure} />
            <Route path='/sp/folders/:libraryId/:nodeId' component={SPBrowseFolderStructure} />
            <Route path='/sp/albums/:id' component={SPAlbum} />
            <Route path='/sp/player' component={SPPlayer} />
          </Route>
        </Route>
        <Route path='/init_login' component={InitLogin} />
        <Route path='*404' component={AppError} />
      </Route>
    </Router>
  );
}
