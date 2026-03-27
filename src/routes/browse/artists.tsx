import Heading2 from '~/components/common/Heading2';

function BrowseArtists() {
  return (
    <div class='flex flex-col gap-4 p-3 h-full w-full overflow-x-hidden overflow-y-auto bg-zinc-100'>
      <Heading2>Artist</Heading2>
      <p class='text-sm text-zinc-500'>Select an artist from the sidebar to view their albums.</p>
    </div>
  );
}

export default BrowseArtists;
