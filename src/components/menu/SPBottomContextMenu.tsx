import { Component, createEffect, createSignal, For, on, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { closeContextMenu, contextMenu, type ContextMenuState } from '~/stores/ContextMenuStore';
import { isSP } from '~/utils/isSP';
import MenuItem from './MenuItem';

const TRANSITION_DURATION = 220;
const DISMISS_DISTANCE = 80; // px: この距離以上スワイプで dismiss
const DISMISS_VELOCITY = 600; // px/s: この速度以上で dismiss

const SPBottomContextMenu: Component = () => {
  const [visible, setVisible] = createSignal<ContextMenuState | null>(null);
  const [entered, setEntered] = createSignal(false);

  // ドラッグ状態
  const [dragOffset, setDragOffset] = createSignal(0);
  const [isDragging, setIsDragging] = createSignal(false);
  let sheetRef: HTMLDivElement | undefined;
  let startY = 0;
  let lastY = 0;
  let lastTime = 0;
  let velocity = 0;

  createEffect(
    on(contextMenu, (state) => {
      if (!isSP()) return;
      if (state) {
        setVisible(state);
        requestAnimationFrame(() => setEntered(true));
      } else {
        setEntered(false);
        setTimeout(() => setVisible(null), TRANSITION_DURATION);
      }
    })
  );

  function dismiss() {
    setDragOffset(0);
    setIsDragging(false);
    setEntered(false);
    setTimeout(() => {
      closeContextMenu();
      setVisible(null);
    }, TRANSITION_DURATION);
  }

  function snapBack() {
    setDragOffset(0);
    setIsDragging(false);
  }

  function onPointerDown(e: PointerEvent) {
    startY = e.clientY;
    lastY = e.clientY;
    lastTime = e.timeStamp;
    velocity = 0;
    setIsDragging(true);
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!isDragging()) return;
    const delta = e.clientY - startY;
    if (delta < 0) return; // 上方向は無視
    const now = e.timeStamp;
    const dt = now - lastTime;
    if (dt > 0) velocity = ((e.clientY - lastY) / dt) * 1000;
    lastY = e.clientY;
    lastTime = now;
    setDragOffset(delta);
  }

  function onPointerUp() {
    if (!isDragging()) return;
    if (dragOffset() >= DISMISS_DISTANCE || velocity >= DISMISS_VELOCITY) {
      dismiss();
    } else {
      snapBack();
    }
  }

  return (
    <Show when={isSP() && visible()}>
      {(state) => (
        <Portal>
          <div
            class='fixed inset-0 z-120 flex flex-col justify-end'
            style={{
              'background-color': entered() ? 'rgba(0,0,0,0.5)' : 'rgba(0,0,0,0)',
              transition: isDragging() ? 'none' : `background-color ${TRANSITION_DURATION}ms ease-out`,
            }}
            onClick={(e) => {
              if (e.target === e.currentTarget) dismiss();
            }}
          >
            <div
              ref={sheetRef}
              class='bg-primary-surface w-full rounded-t-xl'
              style={{
                transform: entered() ? `translateY(${dragOffset()}px)` : 'translateY(100%)',
                transition: isDragging() ? 'none' : `transform ${TRANSITION_DURATION}ms ease-out`,
                'padding-bottom': 'env(safe-area-inset-bottom, 0px)',
              }}
            >
              {/* ハンドル: ここをドラッグで閉じられる */}
              <div
                class='flex w-full cursor-grab justify-center py-3 active:cursor-grabbing'
                style={{ 'touch-action': 'none' }}
                onPointerDown={onPointerDown}
                onPointerMove={onPointerMove}
                onPointerUp={onPointerUp}
                onPointerCancel={snapBack}
              >
                <div class='bg-secondary-border h-1 w-10 rounded-full' />
              </div>
              <div class='pb-4'>
                <For each={state().items}>
                  {(item) => (
                    <MenuItem
                      icon={item.icon}
                      label={item.label}
                      disabled={item.disabled}
                      onClick={() => {
                        item.onClick();
                        dismiss();
                      }}
                    />
                  )}
                </For>
              </div>
            </div>
          </div>
        </Portal>
      )}
    </Show>
  );
};

export default SPBottomContextMenu;
