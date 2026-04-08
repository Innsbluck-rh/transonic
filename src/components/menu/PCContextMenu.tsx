import { Component, createEffect, For, on, onCleanup, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { closeContextMenu, contextMenu } from '~/stores/ContextMenuStore';
import { isSP } from '~/utils/isSP';
import MenuItem from './MenuItem';

const PCContextMenu: Component = () => {
  let menuRef: HTMLDivElement | undefined;

  // メニュー表示中のみ外側クリック・Escリスナーを登録
  createEffect(
    on(contextMenu, (state) => {
      if (!state || isSP()) return;

      const onPointerDown = (e: PointerEvent) => {
        if (menuRef && !menuRef.contains(e.target as Node)) {
          closeContextMenu();
        }
      };

      const onKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          closeContextMenu();
        }
      };

      // 1フレーム遅延させて、メニューを開いたクリック自体で閉じないようにする
      requestAnimationFrame(() => {
        document.addEventListener('pointerdown', onPointerDown);
        document.addEventListener('keydown', onKeyDown);
      });

      onCleanup(() => {
        document.removeEventListener('pointerdown', onPointerDown);
        document.removeEventListener('keydown', onKeyDown);
      });
    })
  );

  // viewportクランプ: メニュー要素の描画後に位置を補正
  createEffect(
    on(contextMenu, (state) => {
      if (!state || isSP() || !menuRef) return;

      requestAnimationFrame(() => {
        if (!menuRef) return;
        const rect = menuRef.getBoundingClientRect();
        const maxX = window.innerWidth - rect.width - 8;
        const maxY = window.innerHeight - rect.height - 8;

        menuRef.style.left = `${Math.max(4, Math.min(state.anchorX, maxX))}px`;
        menuRef.style.top = `${Math.max(4, Math.min(state.anchorY, maxY))}px`;
        menuRef.style.opacity = '1';
      });
    })
  );

  return (
    <Show when={!isSP() && contextMenu()}>
      {(state) => (
        <Portal>
          <div
            ref={menuRef}
            class='border-secondary-border bg-primary-surface fixed z-[110] min-w-44 rounded-md border py-1 shadow-lg'
            style={{ left: `${state().anchorX}px`, top: `${state().anchorY}px`, opacity: '0' }}
          >
            <For each={state().items}>
              {(item) => (
                <MenuItem
                  icon={item.icon}
                  label={item.label}
                  disabled={item.disabled}
                  onClick={() => {
                    item.onClick();
                    closeContextMenu();
                  }}
                />
              )}
            </For>
          </div>
        </Portal>
      )}
    </Show>
  );
};

export default PCContextMenu;
