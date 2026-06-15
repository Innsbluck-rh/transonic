import { For } from 'solid-js';
import { setAppearanceSetting, settingsStore } from '~/features/settings/service';
import { applyTheme, currentTheme } from '~/features/theme/service';
import Heading3 from '../common/text/Heading3';
import SettingSection from './SettingSection';

const themeOptions = [
  { value: 'trans', label: 'Transonic' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
] as const;

const albumDisplayOptions = [
  { value: 'grid', label: 'Grid' },
  { value: 'list', label: 'List' },
] as const;

function AppearanceSettings() {
  return (
    <SettingSection title='Appearance'>
      <Heading3>Theme</Heading3>

      <div class='flex flex-wrap gap-2'>
        <For each={themeOptions}>
          {(option) => (
            <button
              type='button'
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

      <Heading3>Album Layout</Heading3>

      <div class='flex flex-wrap gap-2'>
        <For each={albumDisplayOptions}>
          {(option) => (
            <button
              type='button'
              classList={{
                'bg-primary-selected border-accent text-accent': settingsStore.appearance.albumDisplayMode === option.value,
              }}
              onClick={() => void setAppearanceSetting('albumDisplayMode', option.value)}
            >
              {option.label}
            </button>
          )}
        </For>
      </div>
    </SettingSection>
  );
}

export default AppearanceSettings;
