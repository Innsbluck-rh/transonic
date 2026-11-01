import { Component, Show } from 'solid-js';

interface ArtistRowProps {
  label: string;
  coverArtUrl?: string | null;
  selected?: boolean;
  active?: boolean;
  onClick?: () => void;
  ref?: (element: HTMLDivElement) => void;
}

const ArtistRow: Component<ArtistRowProps> = (props) => {
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
        <Show
          when={props.coverArtUrl}
          fallback={
            <div class='border-primary-border flex size-full items-center justify-center rounded-full border'>
              <i class='icon-[material-symbols--person] text-secondary-text opacity-50' />
            </div>
          }
        >
          {(url) => (
            <div class='relative flex size-full items-center justify-center'>
              {/*<i class='icon-[material-symbols--artist-rounded] text-primary-text absolute right-0 bottom-0 translate-1 text-sm opacity-75' />*/}
              <img src={url()} class='size-full rounded-full object-cover' loading='lazy' decoding='async' />
            </div>
          )}
        </Show>
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

export default ArtistRow;
