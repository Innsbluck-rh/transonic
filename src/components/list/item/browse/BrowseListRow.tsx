import { Component } from 'solid-js';

interface BrowseListRowProps {
  label: string;
  onClick?: () => void;
}

const BrowseListRow: Component<BrowseListRowProps> = (props) => {
  return (
    <div class='w-full cursor-pointer px-3 py-2 hover:bg-zinc-200' onClick={() => props.onClick?.()}>
      <p class='text-xs'>{props.label}</p>
    </div>
  );
};

export default BrowseListRow;
