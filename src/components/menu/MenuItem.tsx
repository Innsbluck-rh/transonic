import { Icon } from '@iconify-icon/solid';
import { Component, Show } from 'solid-js';
import type { MenuItem } from '~/features/menu/types';

const MenuItem: Component<MenuItem> = (props) => {
  return (
    <div
      class='ripple hover:bg-primary-hover flex w-full cursor-pointer flex-row items-center gap-2 px-3 py-3 text-sm lg:py-2.5'
      classList={{
        'text-secondary-text pointer-events-none': props.disabled,
      }}
      onClick={() => {
        if (!props.disabled) props.onClick();
      }}
    >
      <div class='flex w-5 items-center justify-center'>
        <Show when={props.icon}>{(icon) => <Icon icon={icon()} class='scale-150' />}</Show>
      </div>
      <p>{props.label}</p>
    </div>
  );
};

export default MenuItem;
