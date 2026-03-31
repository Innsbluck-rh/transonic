import { Component } from 'solid-js';
import BrowseList, { BrowseListItem } from '~/components/list/BrowseList';

interface AlbumArtistListProps {}

const AlbumArtistList: Component<AlbumArtistListProps> = (props) => {
  const items: BrowseListItem[] = [
    {
      id: '1',
      name: 'test',
    },
  ];

  return <BrowseList items={items} emptyMessage='Nothing Found in Library' />;
};

export default AlbumArtistList;
