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
      class='w-full cursor-pointer px-3 py-2 hover:bg-primary-hover'
      classList={{
        'bg-zinc-200': (props.selected || props.active) ?? false,
      }}
      onClick={() => props.onClick?.()}
    >
      <p
        class='text-xs'
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
