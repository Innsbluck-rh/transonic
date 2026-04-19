import { For, Show } from 'solid-js';
import Heading1 from '~/components/common/Heading1';
import Heading2 from '~/components/common/Heading2';
import { setPlaybackSetting, settingsStore } from '~/features/settings/service';
import { applyTheme, currentTheme } from '~/features/theme/service';
import { sessionStore } from '~/stores/SessionStore';

const themeOptions = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
] as const;

function SettingsRoute() {
  const activeSession = () => sessionStore.activeSession;
  const isGaplessEnabled = () => settingsStore.playback.gaplessPlaybackEnabled;
  const supportsGaplessPlayback = () => sessionStore.playbackCapabilities.gaplessPlayback;

  return (
    <div class='home-surface-root gap-4 p-4 lg:p-5'>
      <Heading1>Settings</Heading1>

      <section class='bg-primary-plane border-primary-border flex flex-col gap-3 rounded-lg border p-4'>
        <div class='flex items-center justify-between gap-3'>
          <Heading2>Appearance</Heading2>
        </div>
        <div class='flex flex-wrap gap-2'>
          <For each={themeOptions}>
            {(option) => (
              <button
                classList={{
                  'bg-primary-selected border-accent text-accent': currentTheme() === option.value,
                }}
                onClick={() => applyTheme(option.value)}
              >
                {option.label}
              </button>
            )}
          </For>
        </div>
      </section>

      <Show when={supportsGaplessPlayback()}>
        <section class='bg-primary-plane border-primary-border flex flex-col gap-4 rounded-lg border p-4'>
          <Heading2>Playback Defaults</Heading2>

          <label class='flex items-center justify-between gap-4'>
            <span>Gapless Playback</span>
            <input
              type='checkbox'
              checked={isGaplessEnabled()}
              class='accent-accent h-4 w-4 border border-current'
              onChange={(event) => setPlaybackSetting('gaplessPlaybackEnabled', event.currentTarget.checked)}
            />
          </label>
        </section>
      </Show>

      <section class='bg-primary-plane border-primary-border flex flex-col gap-3 rounded-lg border p-4'>
        <Heading2>Server</Heading2>
        <Show when={activeSession()} fallback={<p class='text-secondary-text leading-5'>No active server session.</p>}>
          {(session) => (
            <div class='flex flex-col gap-2'>
              <p>{session().username}</p>
              <p class='text-secondary-text leading-5'>{session().normalizedServerUrl}</p>
              <p class='text-secondary-text text-xs'>
                API {session().apiVersion}
                <Show when={session().serverType}>{(serverType) => <> · {serverType()}</>}</Show>
                <Show when={session().serverVersion}>{(serverVersion) => <> {serverVersion()}</>}</Show>
              </p>
            </div>
          )}
        </Show>
      </section>
    </div>
  );
}

export default SettingsRoute;
