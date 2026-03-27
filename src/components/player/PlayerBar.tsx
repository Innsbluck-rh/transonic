import { Component } from 'solid-js';
import PlayerIcon from './PlayerIcon';

const PlayerBar: Component = () => {
  return (
    <div class='flex flex-row w-full px-4 py-2 items-center  border-t border-zinc-600'>
      <PlayerIcon type='prev' />
      <PlayerIcon type='play' />
      <PlayerIcon type='next' />

      <div class='flex flex-col flex-1 justify-center ml-2'>
        <p>title</p>
        <p>artist</p>
      </div>
      <div class='flex flex-col gap-1'>
        <p>00:00 / 00:00</p>
      </div>
    </div>
  );
};

export default PlayerBar;
