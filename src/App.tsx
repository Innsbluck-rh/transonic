// @refresh reload

import { MetaProvider } from '@solidjs/meta';
import { Route, Router } from '@solidjs/router';
import Index from './routes';
import BrowseAlbumArtists from './routes/browse/album_artists';
import BrowseArtistAlbums from './routes/browse/artist_albums';
import BrowseArtists from './routes/browse/artists';
import BrowseFolderStructure from './routes/browse/folder_structure';
import BrowseFolders from './routes/browse/folders';
import Home from './routes/home';
import HomeLayout from './routes/homeLayout';
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
      <Route path='/' component={Index} />
      <Route component={HomeLayout}>
        <Route path='/home' component={Home} />
        <Route path='/browse/folders' component={BrowseFolders} />
        <Route path='/browse/folders/:libraryId' component={BrowseFolderStructure} />
        <Route path='/browse/folders/:libraryId/:nodeId' component={BrowseFolderStructure} />
        <Route path='/browse/artists' component={BrowseArtists} />
        <Route path='/browse/artists/:id' component={BrowseArtistAlbums} />
        <Route path='/browse/album-artists' component={BrowseAlbumArtists} />
      </Route>
      <Route path='/init_login' component={InitLogin} />
    </Router>
  );
}
