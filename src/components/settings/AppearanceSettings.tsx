import { For } from 'solid-js';
import { setAppearanceSetting, settingsStore } from '~/features/settings/service';
import { applyTheme, currentTheme } from '~/features/theme/service';
import SettingSection from './SettingSection';
import SettingSubSection from './SettingSubSection';

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
      <SettingSubSection title='Theme'>
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
      </SettingSubSection>

      <SettingSubSection title='Album Layout'>
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
      </SettingSubSection>
    </SettingSection>
  );
}

export default AppearanceSettings;
