import { Component, Show } from 'solid-js';
import type { MenuItem } from '~/features/menu/types';

const MenuItem: Component<MenuItem> = (props) => {
  return (
    <div
      class='ripple hover:bg-primary-hover sp:py-3 flex w-full cursor-pointer flex-row items-center gap-2 px-3 py-2.5 text-sm'
      classList={{
        'text-secondary-text pointer-events-none': props.disabled,
      }}
      onClick={() => {
        if (!props.disabled) props.onClick();
      }}
    >
      <div class='flex w-5 items-center justify-center'>
        <Show when={props.icon}>{(icon) => <i class={`${icon()} -m-[3.5px] text-[21px]`} />}</Show>
      </div>
      <p>{props.label}</p>
    </div>
  );
};

export default MenuItem;
