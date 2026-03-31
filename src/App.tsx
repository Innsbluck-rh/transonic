// @refresh reload

import { MetaProvider } from '@solidjs/meta';
import { Route, Router } from '@solidjs/router';
import Index from './routes';
import BrowseArtistAlbums from './routes/browse/artist_albums';
import BrowseFolderStructure from './routes/browse/folder_structure';
import Home from './routes/home';
import HomeConnectionError from './routes/home_errors/connection_error';
import HomeNetworkError from './routes/home_errors/network_error';
import HomeLayout from './routes/homeLayout';
import InitLogin from './routes/init_login';
import NotFound from './routes/notFound';

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
      <Route path='/' component={Index} />
      <Route component={HomeLayout}>
        <Route path='/home' component={Home} />
        <Route path='/home/network_error' component={HomeNetworkError} />
        <Route path='/home/connection_error' component={HomeConnectionError} />
        <Route path='/browse/folders/:libraryId' component={BrowseFolderStructure} />
        <Route path='/browse/folders/:libraryId/:nodeId' component={BrowseFolderStructure} />
        <Route path='/browse/artists/:id' component={BrowseArtistAlbums} />
      </Route>
      <Route path='/init_login' component={InitLogin} />
      <Route path='*404' component={NotFound} />
    </Router>
  );
}
