import { Accessor, createResource, Resource } from 'solid-js';
import { AlbumListContext, AlbumListItem, commands } from '~/bindings';

export function useHomeAlbumList(
  context: AlbumListContext,
  size: number
): {
  albums: Resource<AlbumListItem[]>;
  initLoading: Accessor<boolean>;
  loading: Accessor<boolean>;
  error: Accessor<string | undefined>;
  refetch: () => void;
} {
  const [albums, { refetch }] = createResource(async () => {
    const result = await commands.getAlbumList({
      context: context,
      size: size,
      offset: null,
      fromYear: null,
      toYear: null,
      genre: null,
      musicFolderId: null,
    });

    if (result.status === 'error') throw result.error;
    return result.data.albums;
  });

  // initLoading will be used to determine page loading state (which does not include per-context refetch.)
  // state becomes "refreshing" (not "pending") on refetch once the first fetch has settled, even if it failed.
  const initLoading = () => albums.state === 'unresolved' || albums.state === 'pending';
  const loading = () => albums.loading;
  // a thrown string is wrapped into Error by createResource.
  const error = () => albums.error?.message;

  return { albums, initLoading, loading, error, refetch };
}
