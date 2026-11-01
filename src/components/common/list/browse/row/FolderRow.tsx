import { Component } from 'solid-js';

interface FolderRowProps {
  label: string;
  selected?: boolean;
  active?: boolean;
  onClick?: () => void;
  ref?: (element: HTMLDivElement) => void;
}

const FolderRow: Component<FolderRowProps> = (props) => {
  return (
    <div
      ref={props.ref}
      class='ripple hover:bg-primary-hover flex w-full cursor-pointer flex-row items-center px-2.5 py-2'
      classList={{
        'bg-primary-selected': (props.selected || props.active) ?? false,
      }}
      onClick={() => props.onClick?.()}
    >
      <div class='sp:mr-3.5 sp:size-8 mr-3 flex size-6 shrink-0 items-center justify-center'>
        <i class='icon-[material-symbols--folder-outline] text-secondary-text sp:text-xl text-lg' />
      </div>
      <p
        class='archivo sp:text-sm text-xs'
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

export default FolderRow;
