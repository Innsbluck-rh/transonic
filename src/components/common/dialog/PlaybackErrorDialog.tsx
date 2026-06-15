import { Icon } from '@iconify-icon/solid';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import { usePlaybackStatus } from '~/features/playback/usePlaybackStatus';

export default function PlaybackErrorDialog() {
  const [visibleError, setVisibleError] = createSignal<string | null>(null);
  const [dismissedError, setDismissedError] = createSignal<string | null>(null);
  const [copyState, setCopyState] = createSignal<string | null>(null);
  const playbackStatus = usePlaybackStatus();
  const playbackError = createMemo(() => {
    const error = playbackStatus()?.playbackError ?? null;
    return error && !error.handled ? error.message : null;
  });

  createEffect(() => {
    const error = playbackError();
    if (!error) {
      setVisibleError(null);
      setDismissedError(null);
      setCopyState(null);
      return;
    }

    if (error !== dismissedError()) {
      setVisibleError(error);
      setCopyState(null);
    }
  });

  const close = () => {
    const error = visibleError();
    if (error) {
      setDismissedError(error);
    }
    setVisibleError(null);
    setCopyState(null);
  };

  const copy = async () => {
    const error = visibleError();
    if (!error) {
      return;
    }

    try {
      await navigator.clipboard.writeText(error);
      setCopyState('Copied!');
    } catch {
      setCopyState('Copy failed');
    }
  };

  return (
    <Show when={visibleError()}>
      {(error) => (
        <div class='fixed inset-0 z-100 flex items-center justify-center bg-black/45 p-4' role='presentation'>
          <section
            class='border-primary-border bg-primary-plane text-primary-text flex max-h-[80dvh] w-full max-w-2xl flex-col overflow-hidden rounded border shadow-xl'
            role='alertdialog'
            aria-modal='true'
            aria-labelledby='playback-error-title'
          >
            <header class='border-secondary-border flex items-center gap-3 border-b px-4 py-3'>
              <Icon class='text-xl text-red-500' icon='pixelarticons:square-alert' />
              <h2 id='playback-error-title' class='text-primary-text min-w-0 flex-1 text-base'>
                Playback error
              </h2>
              <button class='button-borderless px-2' type='button' onClick={close} aria-label='Close playback error'>
                <Icon icon='material-symbols:close' />
              </button>
            </header>
            <pre class='text-primary-text overflow-auto p-4 text-xs leading-5 break-words whitespace-pre-wrap'>{error()}</pre>
            <footer class='border-secondary-border flex items-center justify-end gap-2 border-t px-4 py-3'>
              <Show when={copyState()}>{(state) => <span class='text-secondary-text mr-auto text-xs'>{state()}</span>}</Show>
              <button class='flex items-center gap-1' type='button' onClick={copy}>
                <Icon icon='material-symbols:content-copy-outline' />
                <span>Copy</span>
              </button>
              <button type='button' onClick={close}>
                Close
              </button>
            </footer>
          </section>
        </div>
      )}
    </Show>
  );
}
