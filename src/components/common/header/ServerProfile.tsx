import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import type { SavedProfileSummary } from '~/bindings';
import { deleteProfile } from '~/features/session/profile';

interface ProfilesProps {
  profiles: SavedProfileSummary[];
}

const ProfilesNew: Component<ProfilesProps> = (props) => {
  const activeProfile = createMemo(() => props.profiles.find((p) => p.isActive));
  const [open, setOpen] = createSignal(false);

  let triggerRef: HTMLDivElement;
  let popupRef: HTMLDivElement;

  const handlePopupOutsideClick = (e: MouseEvent) => {
    if (open() && !popupRef.contains(e.target as Node) && !triggerRef.contains(e.target as Node)) {
      setOpen(false);
    }
  };
  onMount(() => {
    window.addEventListener('click', handlePopupOutsideClick);
  });
  onCleanup(() => {
    window.removeEventListener('click', handlePopupOutsideClick);
  });

  return (
    <div class='overflow-visible relative'>
      <div
        ref={(ref) => (triggerRef = ref)}
        class='flex flex-row cursor-pointer items-center p-2 gap-2 rounded-full bg-primary-plane hover:bg-primary-hover'
        onClick={() => setOpen(!open())}
      >
        <Icon icon='pixelarticons:server' />
        {/* <Show when={sessionStore.activeSession}>
          <p class='text-primary-text text-xs font-normal'>{activeProfile()?.displayName}</p>
        </Show> */}
        {/* <div
          class='opacity-50 w-0 h-0 
            border-l-[5px] border-l-transparent 
            border-r-[5px] border-r-transparent 
            border-t-[5px] border-t-zinc-800'
        ></div> */}
      </div>

      <Show when={open()}>
        <div
          ref={(ref) => (popupRef = ref)}
          class='absolute right-0 flex flex-col z-10 px-3 py-2 bg-primary-plane border border-primary-border rounded-md shadow'
        >
          <div class='flex flex-col items-end'>
            <p>{activeProfile()?.displayName}</p>
            <p class='text-xs text-secondary-text'>
              {activeProfile()?.normalizedServerUrl} ({activeProfile()?.username})
            </p>
            <a
              class='underline text-red-500 text-xs w-fit'
              href='#'
              onClick={() => {
                const profileToDelete = activeProfile();
                if (profileToDelete) deleteProfile(profileToDelete.profileId);
              }}
            >
              delete this profile
            </a>
          </div>
          <Show when={props.profiles.length > 1}>
            <p class='archivo font-black text-xs'>others</p>
            <For each={props.profiles}>
              {(profile, i) => {
                if (i() === 0) return <></>;
                return (
                  <div class='flex flex-row'>
                    <Icon icon='pixelarticons:avatar-circle' />
                    <p>
                      {profile.username} ({profile.displayName})
                    </p>
                  </div>
                );
              }}
            </For>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default ProfilesNew;
