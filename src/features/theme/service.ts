import { createSignal } from 'solid-js';
import { builtInThemes, type ColorTheme, type ThemeVariables, type TransonicColorTheme } from './themes';

const THEME_STORAGE_KEY = 'transonic.color-theme';
const THEME_STYLE_ELEMENT_ID = 'transonic-theme-overrides';

export type AppliedColorTheme = TransonicColorTheme | 'custom';

export type { ColorTheme, ThemeVariables, TransonicColorTheme } from './themes';
export { builtInThemes };

export const [currentTheme, setCurrentTheme] = createSignal<AppliedColorTheme>('light');

let themeOverridesSheet: CSSStyleSheet | null = null;

function isNamedTheme(theme: string | null): theme is TransonicColorTheme {
  return theme === 'light' || theme === 'dark';
}

function readStoredTheme(): TransonicColorTheme {
  if (typeof window === 'undefined') return 'light';

  const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
  return isNamedTheme(storedTheme) ? storedTheme : 'light';
}

function serializeThemeVariables(variables: ThemeVariables): string {
  const declarations = Object.entries(variables);

  if (!declarations.length) return '';

  const body = declarations.map(([name, value]) => `  ${name}: ${value};`).join('\n');
  return `:root {\n${body}\n}`;
}

function ensureThemeOverridesTarget(): CSSStyleSheet | HTMLStyleElement | null {
  if (typeof document === 'undefined') return null;

  if ('adoptedStyleSheets' in document && typeof CSSStyleSheet !== 'undefined') {
    themeOverridesSheet ??= new CSSStyleSheet();

    if (!document.adoptedStyleSheets.includes(themeOverridesSheet)) {
      document.adoptedStyleSheets = [...document.adoptedStyleSheets, themeOverridesSheet];
    }

    return themeOverridesSheet;
  }

  let styleElement = document.getElementById(THEME_STYLE_ELEMENT_ID) as HTMLStyleElement | null;
  if (!styleElement) {
    styleElement = document.createElement('style');
    styleElement.id = THEME_STYLE_ELEMENT_ID;
    document.head.append(styleElement);
  }

  return styleElement;
}

function writeThemeOverrides(theme: ColorTheme) {
  const target = ensureThemeOverridesTarget();
  if (!target) return;

  const cssText = serializeThemeVariables(theme.variables);
  if (target instanceof HTMLStyleElement) {
    target.textContent = cssText;
    return;
  }

  target.replaceSync(cssText);
}

function syncDocumentTheme(themeName: AppliedColorTheme, colorScheme: NonNullable<ColorTheme['colorScheme']>) {
  if (typeof document === 'undefined') return;

  document.documentElement.dataset.theme = themeName;
  document.documentElement.style.colorScheme = colorScheme;
}

export function applyThemeDefinition(
  theme: ColorTheme,
  options: {
    persist?: boolean;
    themeName?: AppliedColorTheme;
  } = {}
) {
  const nextThemeName = options.themeName ?? 'custom';

  writeThemeOverrides(theme);
  syncDocumentTheme(nextThemeName, theme.colorScheme ?? 'light');
  setCurrentTheme(nextThemeName);

  if (!options.persist || typeof window === 'undefined') return;

  if (nextThemeName === 'custom') {
    window.localStorage.removeItem(THEME_STORAGE_KEY);
    return;
  }

  window.localStorage.setItem(THEME_STORAGE_KEY, nextThemeName);
}

export function applyTheme(theme: TransonicColorTheme) {
  applyThemeDefinition(builtInThemes[theme], { persist: true, themeName: theme });
}

export function initializeTheme() {
  const storedTheme = readStoredTheme();
  applyThemeDefinition(builtInThemes[storedTheme], { themeName: storedTheme });
}
