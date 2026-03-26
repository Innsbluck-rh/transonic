import { Component, Show } from 'solid-js';

interface PseudoAlbumCoverProps {
  coverArtId?: string | null;
  year?: number | null;
}

const PseudoAlbumCover: Component<PseudoAlbumCoverProps> = (props) => {
  return (
    <div class='flex h-28 w-28 items-end justify-between border border-zinc-200 bg-gradient-to-br from-zinc-100 via-white to-zinc-200 p-2'>
      <div class='min-w-0'>
        <p class='text-[10px] font-semibold uppercase tracking-[0.2em] text-zinc-400'>album</p>
        <Show when={props.coverArtId}>
          <p class='mt-1 truncate text-[10px] text-zinc-500'>cover #{props.coverArtId?.slice(0, 8)}</p>
        </Show>
      </div>
      <Show when={props.year}>
        <p class='text-[10px] font-semibold text-zinc-500'>{props.year}</p>
      </Show>
    </div>
  );
};

export default PseudoAlbumCover;
