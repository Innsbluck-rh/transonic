import { Component, createMemo, createSignal, onCleanup, onMount, Show } from 'solid-js';
import type { SavedProfileSummary } from '~/bindings';
import { deleteProfile } from '~/features/session/profile';
import { sessionStore } from '~/stores/SessionStore';

interface ProfilesProps {
  profiles: SavedProfileSummary[];
}

const ProfilesNew: Component<ProfilesProps> = (props) => {
  const first = createMemo(() => props.profiles[0]);
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
        class='flex flex-row cursor-pointer items-center select-none px-2 py-1 gap-2 border border-zinc-500 rounded-md bg-zinc-50 hover:bg-zinc-200'
        onClick={() => setOpen(!open())}
      >
        <Show when={sessionStore.activeSession}>
          <p class='text-zinc-900 text-xs font-normal'>
            {sessionStore.activeSession!.username} ({first().displayName})
          </p>
        </Show>
        <div
          class='opacity-50 w-0 h-0 
            border-l-[5px] border-l-transparent 
            border-r-[5px] border-r-transparent 
            border-t-[5px] border-t-zinc-800'
        ></div>
      </div>

      <Show when={open()}>
        <div
          ref={(ref) => (popupRef = ref)}
          class='absolute right-0 mt-1 flex flex-col text-end items-end px-3 py-2 border border-zinc-500 rounded-md bg-zinc-50 shadow'
        >
          <p>
            {first().username} ({first().displayName})
          </p>
          <p class='text-xs text-zinc-500'>{first().normalizedServerUrl}</p>
          <a class='underline text-red-500 text-xs w-fit' href='#' onClick={() => deleteProfile(first().profileId)}>
            delete this profile
          </a>
        </div>
      </Show>
    </div>
  );
};

export default ProfilesNew;
