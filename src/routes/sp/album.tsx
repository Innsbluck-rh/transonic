import { useParams } from '@solidjs/router';

function SPAlbum() {
  const params = useParams();

  return (
    <div class='home-surface-root p-3'>
      <p class='text-sm text-zinc-200'>Album page is not implemented yet.</p>
      <p class='mt-1 text-xs text-zinc-500'>{decodeURIComponent(params.id || '')}</p>
    </div>
  );
}

export default SPAlbum;
