import { Icon } from '@iconify-icon/solid';
import { Component } from 'solid-js';

type PlayerIconTypes = 'play' | 'pause' | 'next' | 'prev';

interface PlayerIconProps {
  type: PlayerIconTypes;
}

const PlayerIcon: Component<PlayerIconProps> = (props) => {
  return (
    <div class='flex items-center justify-center p-2 cursor-pointer rounded-full hover:bg-zinc-200'>
      <Icon icon={`${getIconForType(props.type)}`} class='text-zinc-700 scale-150' />
    </div>
  );
};

function getIconForType(type: PlayerIconTypes) {
  switch (type) {
    case 'play':
      return 'pixelarticons:play';
    case 'pause':
      return 'pixelarticons:pause';
    case 'next':
      return 'pixelarticons:next';
    case 'prev':
      return 'pixelarticons:prev';
  }
}

export default PlayerIcon;
