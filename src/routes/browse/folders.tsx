import Heading2 from '~/components/common/Heading2';

function BrowseFolders() {
  return (
    <div class='flex flex-col gap-4 p-3 h-full w-full overflow-x-hidden overflow-y-auto bg-zinc-100'>
      <Heading2>Folder Structure</Heading2>
      <p class='text-sm text-zinc-500'>Select a music folder from the sidebar to browse its contents.</p>
    </div>
  );
}

export default BrowseFolders;
