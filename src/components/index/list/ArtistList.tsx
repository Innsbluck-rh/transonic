import { Component } from 'solid-js';
import BrowseList, { BrowseListItem } from '~/components/list/BrowseList';

interface ArtistListProps {}

const ArtistList: Component<ArtistListProps> = (props) => {
  const items: BrowseListItem[] = [
    {
      id: '1',
      name: 'test',
    },
  ];

  return <BrowseList items={items} emptyMessage='Nothing Found in Library' />;
};

export default ArtistList;
