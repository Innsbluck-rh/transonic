import { Component } from 'solid-js';

interface QueueMoveGroupItemProps {
  /** How many queue rows are being moved together. */
  count: number;
}

/**
 * Stands for several queue rows dragged together, drawn where they would land.
 * h-13 (52px) is the height of a song row (`SongItem` with an artist line); pl-24
 * puts the text where a song row's text starts (w-13 leading column + w-8 art + mr-3).
 * Only a selection moves several rows at once, so it has the selected look.
 */
const QueueMoveGroupItem: Component<QueueMoveGroupItemProps> = (props) => (
  <div class='song-item relative flex h-13 w-full items-center pl-24' data-selected='true'>
    <p class='song-item-title font-bold'>[{props.count} items]</p>
  </div>
);

export default QueueMoveGroupItem;
