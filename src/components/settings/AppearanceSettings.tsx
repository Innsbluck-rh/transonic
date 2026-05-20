import { For } from 'solid-js';
import { applyTheme, currentTheme } from '~/features/theme/service';
import SettingSection from './SettingSection';

const themeOptions = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
  { value: 'trans', label: 'Transonic' },
] as const;

function AppearanceSettings() {
  return (
    <SettingSection title='Appearance'>
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
    </SettingSection>
  );
}

export default AppearanceSettings;
