import { Component, createEffect, createSignal, For, on, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { closeContextMenu, contextMenu, type ContextMenuState } from '~/stores/ContextMenuStore';
import { isSP } from '~/utils/isSP';
import MenuItem from './MenuItem';

const TRANSITION_DURATION = 220;

const SPBottomContextMenu: Component = () => {
  // アニメーション制御用: contextMenuのsignalとは別に、dismiss中もDOMを維持する
  const [visible, setVisible] = createSignal<ContextMenuState | null>(null);
  const [entered, setEntered] = createSignal(false);

  // 表示: signalがセットされたらvisible化 → 次フレームでスライドイン
  createEffect(
    on(contextMenu, (state) => {
      if (!isSP()) return;

      if (state) {
        setVisible(state);
        requestAnimationFrame(() => {
          setEntered(true);
        });
      } else {
        // closeContextMenu()が外部から呼ばれた場合の即時クリーン
        setEntered(false);
        setTimeout(() => setVisible(null), TRANSITION_DURATION);
      }
    })
  );

  function dismiss() {
    setEntered(false);
    setTimeout(() => {
      closeContextMenu();
      setVisible(null);
    }, TRANSITION_DURATION);
  }

  return (
    <Show when={isSP() && visible()}>
      {(state) => (
        <Portal>
          <div
            class='fixed inset-0 z-120 flex flex-col justify-end'
            style={{
              'background-color': entered() ? 'rgba(0,0,0,0.5)' : 'rgba(0,0,0,0)',
              transition: `background-color ${TRANSITION_DURATION}ms ease-out`,
            }}
            onClick={(e) => {
              // backdrop tap
              if (e.target === e.currentTarget) dismiss();
            }}
          >
            <div
              class='bg-primary-surface min-h-50 w-full rounded-t-xl'
              style={{
                transform: entered() ? 'translateY(0)' : 'translateY(100%)',
                transition: `transform ${TRANSITION_DURATION}ms ease-out`,
                'padding-bottom': 'env(safe-area-inset-bottom, 0px)',
              }}
            >
              <div class='bg-secondary-border mx-auto mt-3 mb-1 h-1 w-10 rounded-full' />
              <div class='py-1'>
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
