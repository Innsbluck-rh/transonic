import { DropdownMenu } from '@kobalte/core/dropdown-menu';
import { Component, createMemo } from 'solid-js';
import { deleteProfile } from '~/features/session/profile';
import { SavedProfileSummary } from '~/models/session';

interface ProfilesProps {
  profiles: SavedProfileSummary[];
}

const Profiles: Component<ProfilesProps> = (props) => {
  const first = createMemo(() => props.profiles[0]);
  return (
    <DropdownMenu>
      <DropdownMenu.Trigger class='flex flex-row cursor-pointer items-center px-2 py-1 gap-2 border border-zinc-500 rounded-md bg-zinc-50 hover:bg-zinc-200'>
        <p class='text-zinc-900 text-xs font-normal'>
          {first().username} ({first().displayName})
        </p>
        <DropdownMenu.Icon>
          <div
            class='opacity-50 w-0 h-0 
            border-l-[5px] border-l-transparent 
            border-r-[5px] border-r-transparent 
            border-t-[5px] border-t-zinc-800'
          ></div>
        </DropdownMenu.Icon>
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content class='flex flex-col text-end items-end mt-1 px-3 py-2 border border-zinc-500 rounded-md bg-zinc-50 shadow'>
          <p>
            {first().username} ({first().displayName})
          </p>
          <p class='text-xs text-zinc-500'>{first().normalizedServerUrl}</p>
          <a class='underline text-red-500 text-xs w-fit' href='#' onClick={() => deleteProfile(first().profileId)}>
            delete this profile
          </a>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu>
  );
};

export default Profiles;
