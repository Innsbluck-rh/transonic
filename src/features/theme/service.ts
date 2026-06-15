import { createSignal } from 'solid-js';
import { DEFAULT_COLOR_THEME, builtInThemes, type ColorTheme, type ThemeVariables, type TransonicColorTheme } from './themes';

const THEME_STORAGE_KEY = 'transonic.color-theme';
const THEME_STYLE_ELEMENT_ID = 'transonic-theme-overrides';

export type AppliedColorTheme = TransonicColorTheme | 'custom';

export type { ColorTheme, ThemeVariables, TransonicColorTheme } from './themes';
export { builtInThemes };

export const [currentTheme, setCurrentTheme] = createSignal<AppliedColorTheme>(DEFAULT_COLOR_THEME);

let themeOverridesSheet: CSSStyleSheet | null = null;

type SystemBarsBridge = {
  applyTheme(statusBarColor: string, navigationBarColor: string, useDarkSystemBarIcons: boolean): void;
};

declare global {
  interface Window {
    TransonicSystemBars?: SystemBarsBridge;
  }
}

function isNamedTheme(theme: string | null): theme is TransonicColorTheme {
  return theme === 'light' || theme === 'dark' || theme === 'trans';
}

function readStoredTheme(): TransonicColorTheme {
  if (typeof window === 'undefined') return DEFAULT_COLOR_THEME;

  const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
  return isNamedTheme(storedTheme) ? storedTheme : DEFAULT_COLOR_THEME;
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

function toHexByte(value: number): string {
  return Math.max(0, Math.min(255, Math.round(value)))
    .toString(16)
    .padStart(2, '0');
}

function parseCssAlpha(value: string | undefined): number {
  if (!value) return 255;

  const normalized = value.trim();
  if (normalized.endsWith('%')) {
    return (Number(normalized.slice(0, -1)) / 100) * 255;
  }

  const alpha = Number(normalized);
  return alpha <= 1 ? alpha * 255 : alpha;
}

function computedRgbToHex(color: string): string | null {
  const match = color.trim().match(/^rgba?\(\s*([\d.]+)(?:\s*,\s*|\s+)([\d.]+)(?:\s*,\s*|\s+)([\d.]+)(?:\s*(?:,|\/)\s*([\d.]+%?))?\s*\)$/i);

  if (!match) return null;

  const red = toHexByte(Number(match[1]));
  const green = toHexByte(Number(match[2]));
  const blue = toHexByte(Number(match[3]));
  const alpha = toHexByte(parseCssAlpha(match[4]));

  return alpha === 'ff' ? `#${red}${green}${blue}` : `#${alpha}${red}${green}${blue}`;
}

function resolveCssColorToHex(cssColor: string): string | null {
  if (typeof document === 'undefined') return null;

  const probe = document.createElement('span');
  probe.style.color = cssColor;
  probe.style.opacity = '0';
  probe.style.pointerEvents = 'none';
  probe.style.position = 'fixed';

  const parent = document.body ?? document.documentElement;
  parent.append(probe);
  const computedColor = window.getComputedStyle(probe).color;
  probe.remove();

  return computedRgbToHex(computedColor);
}

function syncNativeSystemBars(colorScheme: NonNullable<ColorTheme['colorScheme']>) {
  if (typeof window === 'undefined') return;

  const systemBarColor = resolveCssColorToHex('var(--color-sp-nav-space)');
  if (!systemBarColor) return;

  window.TransonicSystemBars?.applyTheme(systemBarColor, systemBarColor, colorScheme === 'light');
}

export function applyThemeDefinition(
  theme: ColorTheme,
  options: {
    persist?: boolean;
    themeName?: AppliedColorTheme;
  } = {}
) {
  const nextThemeName = options.themeName ?? 'custom';
  const colorScheme = theme.colorScheme;

  writeThemeOverrides(theme);
  syncDocumentTheme(nextThemeName, colorScheme);
  syncNativeSystemBars(colorScheme);
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
