import { Component, createEffect, createMemo, createSignal, For, on, onCleanup } from 'solid-js';

import { BROWSE_MODES, type BrowseMode } from '~/features/navigation/routes';
import styles from './BrowseModeSelect.module.css';

interface BrowseModeSelectProps {
  defaultMode: BrowseMode;
  onSelect?: (mode: BrowseMode) => void;
}

const BrowseModeSelect: Component<BrowseModeSelectProps> = (props) => {
  let rootRef: HTMLDivElement | undefined;
  const browseMode = createMemo(() => props.defaultMode);
  const [open, setOpen] = createSignal(false);
  const otherModes = createMemo<BrowseMode[]>(() => BROWSE_MODES.filter((mode) => mode !== browseMode()));

  // 開いている間のみ外側クリック・Escリスナーを登録
  createEffect(
    on(open, (isOpen) => {
      if (!isOpen) return;

      const onPointerDown = (e: PointerEvent) => {
        if (rootRef && !rootRef.contains(e.target as Node)) {
          setOpen(false);
        }
      };

      const onKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          setOpen(false);
        }
      };

      document.addEventListener('pointerdown', onPointerDown);
      document.addEventListener('keydown', onKeyDown);

      onCleanup(() => {
        document.removeEventListener('pointerdown', onPointerDown);
        document.removeEventListener('keydown', onKeyDown);
      });
    })
  );

  return (
    <div ref={rootRef} class='relative w-full'>
      <div class='bg-primary-surface border-secondary-border flex w-full flex-col overflow-hidden rounded-lg border shadow-md'>
        <div
          class='hover:bg-primary-hover flex w-full cursor-pointer flex-row items-center gap-2 px-3 py-1.5'
          onClick={() => setOpen((current) => !current)}
        >
          {/* 全モードを同じセルに重ね、幅を最長のモード名に固定する（選択中以外は invisible） */}
          <div class='grid min-w-0 flex-1'>
            <For each={BROWSE_MODES}>
              {(mode) => (
                <div
                  class='col-start-1 row-start-1 flex min-w-0 items-center gap-2'
                  classList={{ invisible: mode !== browseMode() }}
                  aria-hidden={mode !== browseMode()}
                >
                  <i class={`${getIconForMode(mode)} text-md`} />
                  <p class='archivo text-md truncate font-bold lg:text-sm'>{mode}</p>
                </div>
              )}
            </For>
          </div>
          <div class='ml-auto flex w-4 text-xl'>
            <i class={open() ? 'icon-[material-symbols--arrow-drop-up]' : 'icon-[material-symbols--arrow-drop-down]'} />
          </div>
        </div>
      </div>

      <div
        class={`bg-primary-surface border-secondary-border absolute top-full right-0 left-0 mt-1 flex flex-col overflow-hidden rounded-lg border shadow-lg ${styles.panel}`}
        classList={{ [styles['is-open']]: open() }}
      >
        <For each={otherModes()}>
          {(mode) => (
            <div
              class='hover:bg-primary-hover flex w-full cursor-pointer items-center gap-2 px-3 py-2'
              onClick={() => {
                props.onSelect?.(mode);
                setOpen(false);
              }}
            >
              <i class={`${getIconForMode(mode)} text-md`} />
              <p class='archivo truncate text-sm'>{mode}</p>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};

const getIconForMode = (mode: BrowseMode) => {
  switch (mode) {
    case 'Folder Structures':
      // return "icon-[material-symbols--folder]";
      return 'icon-[material-symbols--folder-copy-rounded]';
    case 'Artists':
      return 'icon-[material-symbols--artist-rounded]';
  }
};

export default BrowseModeSelect;
