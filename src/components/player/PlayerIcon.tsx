import { Component, createMemo, Show } from 'solid-js';
import LoadCircle from '../common/LoadCircle';

type PlayerIconTypes = 'play' | 'pause' | 'next' | 'prev';

interface PlayerIconProps {
  type: PlayerIconTypes;
  // replace the default sizes of the icon and of the loading circle
  iconClass?: string;
  loadingClass?: string;
  disabled?: boolean;
  loading?: boolean;
  onClick?: (e: MouseEvent) => void;
}

const PlayerIcon: Component<PlayerIconProps> = (props) => {
  const icon = createMemo(() => {
    switch (props.type) {
      case 'play':
        return 'icon-[material-symbols--play-arrow]';
      case 'pause':
        return 'icon-[material-symbols--pause]';
      case 'next':
        return 'icon-[material-symbols--skip-next]';
      case 'prev':
        return 'icon-[material-symbols--skip-previous]';
    }
  });

  return (
    <div
      aria-disabled={props.disabled || undefined}
      class='ripple flex items-center justify-center overflow-hidden rounded-full p-2'
      classList={{
        'cursor-pointer hover:bg-primary-hover': !props.disabled,
      }}
      onClick={(e) => {
        e.stopPropagation();
        if (props.disabled) return;
        props.onClick?.(e);
      }}
    >
      <Show when={!props.loading} fallback={<LoadCircle sizeClass={props.loadingClass} />}>
        <i class={`${icon()} ${props.disabled ? 'text-secondary-text' : 'text-primary-text'} ${props.iconClass ?? '-m-1.5 text-[28px]'}`} />
      </Show>
    </div>
  );
};

export default PlayerIcon;
