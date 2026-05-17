import { Component } from 'solid-js';

interface BrowseListRowProps {
  label: string;
  selected?: boolean;
  active?: boolean;
  onClick?: () => void;
}

const BrowseListRow: Component<BrowseListRowProps> = (props) => {
  return (
    <div
      class='ripple hover:bg-primary-hover w-full cursor-pointer px-3.5 py-3 lg:py-2.5'
      classList={{
        'bg-primary-selected': (props.selected || props.active) ?? false,
      }}
      onClick={() => props.onClick?.()}
    >
      <p
        class='text-sm lg:text-xs'
        classList={{
          'text-accent font-bold': props.active ?? false,
          'text-zinc-950': props.selected ?? false,
        }}
      >
        {props.label}
      </p>
    </div>
  );
};

export default BrowseListRow;
