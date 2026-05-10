import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import { commands, type CoverArtCacheStatus } from '~/bindings';
import { sessionStore } from '~/stores/SessionStore';
import SettingSection from './SettingSection';

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }

  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function AlbumArtCacheSettings() {
  const activeSession = () => sessionStore.activeSession;
  const [coverArtCacheStatus, setCoverArtCacheStatus] = createSignal<CoverArtCacheStatus | null>(null);
  const [coverArtCacheMessage, setCoverArtCacheMessage] = createSignal<string | null>(null);
  const [coverArtCacheBusy, setCoverArtCacheBusy] = createSignal(false);
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

  const clearCoverArtCache = async () => {
    setCoverArtCacheBusy(true);
    setCoverArtCacheMessage(null);
    try {
      const result = await commands.clearCoverArtCache();
      if (result.status === 'error') {
        setCoverArtCacheMessage(result.error);
        return;
      }

      setCoverArtCacheStatus(result.data);
    } catch (error) {
      setCoverArtCacheMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setCoverArtCacheBusy(false);
    }
  };

  createEffect(() => {
    sessionStore.activeSession?.profileId;
    setCoverArtCacheMessage(null);
    void refreshCoverArtCacheStatus();
  });

  return (
    <SettingSection title='Album Art Cache'>
      <p class='text-secondary-text text-xs leading-5'>{coverArtCacheSummary()}</p>
      <div class='flex w-full justify-end'>
        <button type='button' disabled={coverArtCacheBusy() || !activeSession()} onClick={() => void clearCoverArtCache()}>
          Clear Cache
        </button>
      </div>
      <Show when={coverArtCacheMessage()}>{(message) => <p class='text-secondary-text text-xs leading-5'>{message()}</p>}</Show>
    </SettingSection>
  );
}

export default AlbumArtCacheSettings;
