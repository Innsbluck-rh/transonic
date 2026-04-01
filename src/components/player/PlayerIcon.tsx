import { Icon } from '@iconify-icon/solid';
import { Component } from 'solid-js';

type PlayerIconTypes = 'play' | 'pause' | 'next' | 'prev';

interface PlayerIconProps {
  type: PlayerIconTypes;
  disabled?: boolean;
  onClick?: (e: MouseEvent) => void;
}

const PlayerIcon: Component<PlayerIconProps> = (props) => {
  return (
    <div
      class='flex items-center justify-center p-2 rounded-full'
      classList={{
        'cursor-pointer hover:bg-zinc-200': !props.disabled,
      }}
      onClick={(e) => {
        if (props.disabled) return;
        props.onClick?.(e);
      }}
    >
      <Icon icon={`${getIconForType(props.type)}`} class={`${props.disabled ? 'text-zinc-400' : 'text-zinc-700'} scale-150`} />
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
