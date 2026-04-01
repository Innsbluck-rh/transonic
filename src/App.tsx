// @refresh reload

import { MetaProvider } from '@solidjs/meta';
import { Navigate, Route, Router } from '@solidjs/router';
import { HOME_ROUTE } from './features/session/bootstrap';
import BrowseArtistAlbums from './routes/browse/artist_albums';
import BrowseFolderStructure from './routes/browse/folder_structure';
import AppError from './routes/error';
import Home from './routes/home';
import HomeLayout from './routes/homeLayout';
import InitLoad from './routes/init_load';
import InitLogin from './routes/init_login';

export default function App() {
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
      <Route path='/' component={() => <Navigate href={HOME_ROUTE} />} />
      <Route component={InitLoad}>
        <Route path='/home/error' component={AppError} />
        <Route component={HomeLayout}>
          <Route path='/home' component={Home} />
          <Route path='/browse/folders/:libraryId' component={BrowseFolderStructure} />
          <Route path='/browse/folders/:libraryId/:nodeId' component={BrowseFolderStructure} />
          <Route path='/browse/artists/:id' component={BrowseArtistAlbums} />
        </Route>
      </Route>
      <Route path='/init_login' component={InitLogin} />
      <Route path='*404' component={AppError} />
    </Router>
  );
}
