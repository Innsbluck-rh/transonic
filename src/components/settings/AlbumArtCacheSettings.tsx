import { createEffect, createMemo, createSignal, onCleanup, Show } from 'solid-js';
import { commands, type CoverArtCacheStatus } from '~/bindings';
import { clearCoverArtCache } from '~/features/albums/service';
import { sessionStore } from '~/stores/SessionStore';
import SettingSection from './SettingSection';

const CLEAR_RESULT_DISPLAY_MS = 1000;

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }

  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function AlbumArtCacheSettings() {
  const [coverArtCacheStatus, setCoverArtCacheStatus] = createSignal<CoverArtCacheStatus | null>(null);
  const [coverArtCacheMessage, setCoverArtCacheMessage] = createSignal<string | null>(null);
  const [coverArtCacheBusy, setCoverArtCacheBusy] = createSignal(false);
  const [clearResult, setClearResult] = createSignal<'complete' | 'failed' | null>(null);
  let clearResultTimeoutId: number | undefined;
  const coverArtCacheSummary = createMemo(() => {
    const status = coverArtCacheStatus();
    const entryCount = status?.entryCount ?? 0;
    const totalBytes = status?.totalBytes ?? 0;

    return `${entryCount.toLocaleString()} Arts, ${formatBytes(totalBytes)}`;
  });

  const refreshCoverArtCacheStatus = async () => {
    setCoverArtCacheMessage(null);
    try {
      const result = await commands.getCoverArtCacheStatus();
      if (result.status === 'error') {
        setCoverArtCacheMessage(result.error);
        return;
      }

      setCoverArtCacheStatus(result.data);
    } catch (error) {
      setCoverArtCacheMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const showClearResult = (result: 'complete' | 'failed') => {
    window.clearTimeout(clearResultTimeoutId);
    setClearResult(result);
    clearResultTimeoutId = window.setTimeout(() => setClearResult(null), CLEAR_RESULT_DISPLAY_MS);
  };

  const clearCache = async () => {
    setCoverArtCacheBusy(true);
    setCoverArtCacheMessage(null);
    try {
      const result = await clearCoverArtCache();
      if (result.status === 'error') {
        setCoverArtCacheMessage(result.error);
        showClearResult('failed');
        return;
      }

      setCoverArtCacheStatus(result.data);
      showClearResult('complete');
    } catch (error) {
      setCoverArtCacheMessage(error instanceof Error ? error.message : String(error));
      showClearResult('failed');
    } finally {
      setCoverArtCacheBusy(false);
    }
  };

  onCleanup(() => window.clearTimeout(clearResultTimeoutId));

  createEffect(() => {
    sessionStore.activeSession?.profileId;
    setCoverArtCacheMessage(null);
    void refreshCoverArtCacheStatus();
  });

  return (
    <SettingSection title='Album Art Cache'>
      <p class='text-secondary-text text-xs leading-5'>{coverArtCacheSummary()}</p>
      <div class='flex w-full items-center justify-end gap-4'>
        <Show when={clearResult() === 'complete'}>
          <div class='flex items-center'>
            <i class='text-accent icon-[material-symbols--check] mr-1 text-lg' />
            <p class='text-accent font-bold'>complete!</p>
          </div>
        </Show>
        <Show when={clearResult() === 'failed'}>
          <div class='flex items-center'>
            <i class='icon-[material-symbols--error] mr-1 text-lg text-red-400' />
            <p class='font-bold text-red-400'>failed to clear</p>
          </div>
        </Show>
        <button type='button' disabled={coverArtCacheBusy()} onClick={() => void clearCache()}>
          Clear Cache
        </button>
      </div>
      <Show when={coverArtCacheMessage()}>{(message) => <p class='text-secondary-text text-xs leading-5'>{message()}</p>}</Show>
    </SettingSection>
  );
}

export default AlbumArtCacheSettings;
