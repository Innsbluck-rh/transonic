import { Icon } from '@iconify-icon/solid';
import { Component } from 'solid-js';
import { applyTheme, currentTheme } from '~/features/theme/service';

const ThemeToggle: Component = () => {
  const isDarkTheme = () => currentTheme() === 'dark';

  return (
    <div class='relative overflow-visible'>
      <div
        class='bg-primary-plane hover:bg-primary-hover flex cursor-pointer flex-row items-center gap-2 rounded-full p-2'
        onClick={() => applyTheme(isDarkTheme() ? 'light' : 'dark')}
      >
        <Icon class='text-primary-text text-[13px]' icon={isDarkTheme() ? 'pixelarticons:sun' : 'pixelarticons:moon'} />
      </div>
    </div>
  );
};

export default ThemeToggle;
