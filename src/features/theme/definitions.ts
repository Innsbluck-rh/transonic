export type TransonicColorTheme = 'light' | 'dark';

export type ThemeVariableName =
  | '--theme-color-primary-plane'
  | '--theme-color-primary-surface'
  | '--theme-color-primary-inner-surface'
  | '--theme-color-primary-selected'
  | '--theme-color-primary-playing'
  | '--theme-color-primary-hover'
  | '--theme-color-primary-border'
  | '--theme-color-secondary-border'
  | '--theme-color-primary-text'
  | '--theme-color-secondary-text'
  | '--theme-color-sp-nav-space'
  | '--theme-color-accent';

export type ThemeVariables = Partial<Record<ThemeVariableName, string>>;

export interface ColorTheme {
  colorScheme?: 'light' | 'dark';
  variables: ThemeVariables;
}

export const lightTheme: ColorTheme = {
  colorScheme: 'light',
  variables: {},
};

export const darkTheme: ColorTheme = {
  colorScheme: 'dark',
  variables: {
    '--theme-color-primary-plane': 'var(--color-zinc-900)',
    '--theme-color-primary-surface': 'var(--color-zinc-800)',
    '--theme-color-primary-inner-surface': 'var(--color-zinc-700)',
    '--theme-color-primary-selected': 'var(--color-zinc-800)',
    '--theme-color-primary-playing': 'var(--color-zinc-800)',
    '--theme-color-primary-hover': 'var(--color-zinc-700)',
    '--theme-color-primary-border': 'var(--color-zinc-600)',
    '--theme-color-secondary-border': 'var(--color-zinc-700)',
    '--theme-color-primary-text': 'var(--color-zinc-100)',
    '--theme-color-secondary-text': 'var(--color-zinc-400)',
    '--theme-color-sp-nav-space': 'var(--color-black)',
  },
};

export const builtInThemes: Record<TransonicColorTheme, ColorTheme> = {
  light: lightTheme,
  dark: darkTheme,
};
