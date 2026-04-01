import { Icon } from '@iconify-icon/solid';
import { Component, Show } from 'solid-js';
import LoadCircle from '../common/LoadCircle';

type PlayerIconTypes = 'play' | 'pause' | 'next' | 'prev';

interface PlayerIconProps {
  type: PlayerIconTypes;
  disabled?: boolean;
  loading?: boolean;
  onClick?: (e: MouseEvent) => void;
}

const PlayerIcon: Component<PlayerIconProps> = (props) => {
  return (
    <div
      class='flex items-center justify-center p-2 rounded-full'
      classList={{
        'cursor-pointer hover:bg-primary-hover': !props.disabled,
      }}
      onClick={(e) => {
        if (props.disabled) return;
        props.onClick?.(e);
      }}
    >
      <Show when={!props.loading} fallback={<LoadCircle />}>
        <Icon icon={`${getIconForType(props.type)}`} class={`${props.disabled ? 'text-secondary-text' : 'text-primary-text'} scale-150`} />
      </Show>
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
