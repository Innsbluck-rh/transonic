export type TransonicColorTheme = 'light' | 'dark' | 'trans';

export const DEFAULT_COLOR_THEME: TransonicColorTheme = 'trans';

export type ThemeVariableName =
  | '--theme-color-primary-bg'
  | '--theme-color-primary-plane'
  | '--theme-color-primary-surface'
  | '--theme-color-primary-selected'
  | '--theme-color-primary-on-playing'
  | '--theme-color-primary-hover'
  | '--theme-color-primary-border'
  | '--theme-color-secondary-border'
  | '--theme-color-primary-text'
  | '--theme-color-secondary-text'
  | '--theme-color-sp-nav-space'
  | '--theme-color-accent';

export type ThemeVariables = Record<ThemeVariableName, string>;

export interface ColorTheme {
  colorScheme: 'light' | 'dark';
  variables: ThemeVariables;
}

export const lightTheme: ColorTheme = {
  colorScheme: 'light',
  variables: {
    '--theme-color-primary-bg': 'var(--color-zinc-200)',
    '--theme-color-primary-plane': 'var(--color-zinc-50)',
    '--theme-color-primary-surface': 'var(--color-gray-100)',
    '--theme-color-primary-selected': 'var(--color-zinc-200)',
    '--theme-color-primary-on-playing': 'var(--color-zinc-800)',
    '--theme-color-primary-hover': 'var(--color-gray-200)',
    '--theme-color-primary-border': 'rgb(0 0 0 / 0.25)',
    '--theme-color-secondary-border': 'rgb(0 0 0 / 0.1)',
    '--theme-color-primary-text': 'var(--color-gray-700)',
    '--theme-color-secondary-text': 'var(--color-gray-500)',
    '--theme-color-sp-nav-space': 'var(--color-zinc-400)',
    '--theme-color-accent': 'var(--color-sky-600)',
  },
};

export const darkTheme: ColorTheme = {
  colorScheme: 'dark',
  variables: {
    '--theme-color-primary-bg': 'var(--color-zinc-800)',
    '--theme-color-primary-plane': 'var(--color-zinc-900)',
    '--theme-color-primary-surface': 'var(--color-zinc-700)',
    '--theme-color-primary-selected': 'var(--color-zinc-800)',
    '--theme-color-primary-on-playing': 'var(--color-zinc-100)',
    '--theme-color-primary-hover': 'var(--color-gray-700)',
    '--theme-color-primary-border': 'rgb(255 255 255 / 0.4)',
    '--theme-color-secondary-border': 'rgb(255 255 255 / 0.1)',
    '--theme-color-primary-text': 'var(--color-zinc-100)',
    '--theme-color-secondary-text': 'var(--color-zinc-400)',
    '--theme-color-sp-nav-space': 'var(--color-black)',
    '--theme-color-accent': 'var(--color-sky-600)',
  },
};

export const transonicTheme: ColorTheme = {
  colorScheme: 'dark',
  variables: {
    '--theme-color-primary-bg': 'var(--color-gray-900)',
    '--theme-color-primary-plane': 'var(--color-zinc-800)',
    '--theme-color-primary-surface': 'var(--color-gray-800)',
    '--theme-color-primary-selected': 'var(--color-gray-700)',
    '--theme-color-primary-on-playing': 'var(--color-gray-100)',
    '--theme-color-primary-hover': 'var(--color-gray-700)',
    '--theme-color-primary-border': 'rgb(255 255 255 / 0.3)',
    '--theme-color-secondary-border': 'rgb(255 255 255 / 0.1)',
    '--theme-color-primary-text': 'var(--color-gray-100)',
    '--theme-color-secondary-text': 'var(--color-gray-400)',
    '--theme-color-sp-nav-space': 'var(--color-black)',
    '--theme-color-accent': 'var(--color-sky-600)',
  },
};

export const builtInThemes: Record<TransonicColorTheme, ColorTheme> = {
  trans: transonicTheme,
  light: lightTheme,
  dark: darkTheme,
};
