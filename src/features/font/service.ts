/**
 * CJK font fallback ordering based on OS locale.
 *
 * ADR-14 introduced Windows/macOS system fonts for CJK fallback to avoid
 * serif rendering with bold weights. The original order (SC→TC→JP→KR) caused
 * Chinese fonts (e.g. Microsoft YaHei) to cover Japanese-specific glyphs
 * (hiragana, katakana) because YaHei includes those code points.
 *
 * This module detects `navigator.language` and reorders the CJK font chain
 * so that the user's locale-native font is tried first.
 *
 * Noto Sans CJK is included with higher priority than OS system fonts.
 * It is NOT bundled — only referenced by name. If installed locally it will
 * be used; otherwise the browser skips it and falls through to system fonts.
 * Two naming conventions exist ("Noto Sans JP" from Google Fonts individual
 * packages and "Noto Sans CJK JP" from the combined CJK package), so both
 * are listed for maximum compatibility.
 */

const CJK_FONTS_PROPERTY = '--cjk-fonts';

/**
 * Per-locale CJK font priority orders.
 *
 * Within each locale the ordering is:
 *   1. Noto Sans for the user's locale (best quality if installed)
 *   2. Noto Sans for other CJK locales
 *   3. OS system fonts for the user's locale
 *   4. OS system fonts for other CJK locales
 *
 * On each platform absent fonts are simply skipped by the browser.
 */
const CJK_FONT_ORDERS: Record<string, readonly string[]> = {
  ja: [
    // Noto Sans — locale-native first
    'Noto Sans JP',
    'Noto Sans CJK JP',
    'Noto Sans SC',
    'Noto Sans CJK SC',
    'Noto Sans TC',
    'Noto Sans CJK TC',
    'Noto Sans KR',
    'Noto Sans CJK KR',
    // OS system fonts — locale-native first
    'Yu Gothic',
    'Hiragino Sans',
    'Microsoft YaHei',
    'Microsoft JhengHei',
    'PingFang SC',
    'PingFang TC',
    'Malgun Gothic',
  ],
  'zh-Hans': [
    'Noto Sans SC',
    'Noto Sans CJK SC',
    'Noto Sans TC',
    'Noto Sans CJK TC',
    'Noto Sans JP',
    'Noto Sans CJK JP',
    'Noto Sans KR',
    'Noto Sans CJK KR',
    'Microsoft YaHei',
    'Microsoft JhengHei',
    'PingFang SC',
    'PingFang TC',
    'Yu Gothic',
    'Hiragino Sans',
    'Malgun Gothic',
  ],
  'zh-Hant': [
    'Noto Sans TC',
    'Noto Sans CJK TC',
    'Noto Sans SC',
    'Noto Sans CJK SC',
    'Noto Sans JP',
    'Noto Sans CJK JP',
    'Noto Sans KR',
    'Noto Sans CJK KR',
    'Microsoft JhengHei',
    'Microsoft YaHei',
    'PingFang TC',
    'PingFang SC',
    'Yu Gothic',
    'Hiragino Sans',
    'Malgun Gothic',
  ],
  ko: [
    'Noto Sans KR',
    'Noto Sans CJK KR',
    'Noto Sans SC',
    'Noto Sans CJK SC',
    'Noto Sans TC',
    'Noto Sans CJK TC',
    'Noto Sans JP',
    'Noto Sans CJK JP',
    'Malgun Gothic',
    'Microsoft YaHei',
    'Microsoft JhengHei',
    'PingFang SC',
    'PingFang TC',
    'Yu Gothic',
    'Hiragino Sans',
  ],
};

/** Fallback when the locale is not CJK (original ADR-14 order: SC→TC→JP→KR). */
const DEFAULT_CJK_FONTS: readonly string[] = [
  'Noto Sans SC',
  'Noto Sans CJK SC',
  'Noto Sans TC',
  'Noto Sans CJK TC',
  'Noto Sans JP',
  'Noto Sans CJK JP',
  'Noto Sans KR',
  'Noto Sans CJK KR',
  'Microsoft YaHei',
  'Microsoft JhengHei',
  'Yu Gothic',
  'Malgun Gothic',
  'Hiragino Sans',
  'PingFang SC',
  'PingFang TC',
];

/**
 * Map a BCP-47 language tag to one of our CJK locale keys, or `null` if the
 * language is not CJK.
 */
function resolveCjkLocale(lang: string): string | null {
  const lower = lang.toLowerCase();

  if (lower.startsWith('ja')) return 'ja';
  if (lower.startsWith('ko')) return 'ko';

  if (lower.startsWith('zh')) {
    // zh-Hant, zh-TW, zh-HK, zh-MO → Traditional Chinese
    if (/\bhant\b/.test(lower) || /\b(tw|hk|mo)\b/.test(lower)) {
      return 'zh-Hant';
    }
    // zh, zh-Hans, zh-CN, zh-SG → Simplified Chinese
    return 'zh-Hans';
  }

  return null;
}

function toFontFamilyValue(fonts: readonly string[]): string {
  return fonts.map((f) => `'${f}'`).join(', ');
}

/**
 * Detect the user's locale via `navigator.language` and set the
 * `--cjk-fonts` CSS custom property on `:root` so that `font-family`
 * declarations can reference `var(--cjk-fonts)`.
 *
 * Should be called once during application bootstrap, before the first paint
 * if possible.
 */
export function initializeCjkFontOrder(): void {
  if (typeof navigator === 'undefined' || typeof document === 'undefined') return;

  const locale = resolveCjkLocale(navigator.language);
  const fonts = locale !== null ? CJK_FONT_ORDERS[locale] : DEFAULT_CJK_FONTS;
  const value = toFontFamilyValue(fonts ?? DEFAULT_CJK_FONTS);

  document.documentElement.style.setProperty(CJK_FONTS_PROPERTY, value);
}
