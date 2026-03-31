import { Component } from 'solid-js';
import PlayerIcon from './PlayerIcon';

const PlayerBar: Component = () => {
  return (
    <div class='flex flex-row w-full h-18 px-4 gap-2 items-center border-t border-zinc-600'>
      <div class='flex flex-row gap-2'>
        <PlayerIcon type='prev' />
        <PlayerIcon type='play' />
        <PlayerIcon type='next' />
      </div>

      <div class='flex flex-col flex-1 ml-2'>
        <p class='archivo bold'>title</p>
        <p class='archivo italic text-xs leading-none opacity-75'>artist</p>
      </div>
      <div class='flex flex-col gap-1'>
        <p class='archivo text-xs'>00:00 / 00:00</p>
      </div>
    </div>
  );
};

export default PlayerBar;
