import { Show } from 'solid-js';
import { setPlaybackSetting, settingsStore } from '~/features/settings/service';
import { sessionStore } from '~/stores/SessionStore';
import SettingSection from './SettingSection';

function PlaybackSettings() {
  const isGaplessEnabled = () => settingsStore.playback.gaplessPlaybackEnabled;
  const supportsGaplessPlayback = () => sessionStore.playbackCapabilities.gaplessPlayback;

  return (
    <Show when={supportsGaplessPlayback()}>
      <SettingSection title='Playback'>
        <label class='flex items-center justify-between gap-4'>
          <span>Gapless Playback</span>
          <input
            type='checkbox'
            checked={isGaplessEnabled()}
            class='accent-accent h-4 w-4 border border-current'
            onChange={(event) => setPlaybackSetting('gaplessPlaybackEnabled', event.currentTarget.checked)}
          />
        </label>
      </SettingSection>
    </Show>
  );
}

export default PlaybackSettings;
