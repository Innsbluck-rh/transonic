import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, Show } from 'solid-js';
import LoadCircle from '../common/LoadCircle';

type PlayerIconTypes = 'play' | 'pause' | 'next' | 'prev';

interface PlayerIconProps {
  type: PlayerIconTypes;
  iconClass?: string;
  disabled?: boolean;
  loading?: boolean;
  onClick?: (e: MouseEvent) => void;
}

const PlayerIcon: Component<PlayerIconProps> = (props) => {
  const icon = createMemo(() => {
    switch (props.type) {
      case 'play':
        return 'material-symbols:play-arrow';
      case 'pause':
        return 'material-symbols:pause';
      case 'next':
        return 'material-symbols:skip-next';
      case 'prev':
        return 'material-symbols:skip-previous';
    }
  });

  return (
    <div
      class='ripple flex items-center justify-center overflow-visible rounded-full p-2'
      classList={{
        'cursor-pointer hover:bg-primary-hover': !props.disabled,
      }}
      onClick={(e) => {
        e.stopPropagation();
        if (props.disabled) return;
        props.onClick?.(e);
      }}
    >
      <Show when={!props.loading} fallback={<LoadCircle class={props.iconClass} />}>
        <Icon icon={icon()} class={`${props.disabled ? 'text-secondary-text' : 'text-primary-text'} scale-175 ${props.iconClass}`} />
      </Show>
    </div>
  );
};

export default PlayerIcon;
