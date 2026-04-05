import { Icon } from '@iconify-icon/solid';
import { Component, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import type { SavedProfileSummary } from '~/bindings';
import { deleteProfile } from '~/features/session/profile';

interface ServerProfileProps {
  profiles: SavedProfileSummary[];
}

const ServerProfile: Component<ServerProfileProps> = (props) => {
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
    <div class='relative overflow-visible'>
      <div
        ref={(ref) => (triggerRef = ref)}
        class='bg-primary-plane hover:bg-primary-hover flex cursor-pointer flex-row items-center gap-2 rounded-full p-2'
        onClick={() => setOpen(!open())}
      >
        <Icon class='text-primary-text text-[16px]' icon='pixelarticons:server' />
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
          class='bg-primary-plane border-primary-border absolute right-0 z-100 flex flex-col rounded-md border px-3 py-2 shadow'
        >
          <div class='flex flex-col items-end'>
            <p>{activeProfile()?.displayName}</p>
            <p class='text-secondary-text text-xs'>
              {activeProfile()?.normalizedServerUrl} ({activeProfile()?.username})
            </p>
            <a
              class='w-fit text-xs text-red-500 underline'
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
            <p class='archivo text-xs font-black'>others</p>
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

export default ServerProfile;
