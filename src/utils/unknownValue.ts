/**
 * Placeholder shown when a tag that names the item (song title, artist, album
 * name, folder name) has no value. The rule and the per-tag table live in
 * `docs/unavailable-value-fallback.md`; Rust uses the same string for SMTC and
 * the Android media session (`src-tauri/src/unknown_value.rs`).
 */
export const UNKNOWN_VALUE = '[unknown]';

/** The value, or {@link UNKNOWN_VALUE} when it is missing. `null`, `undefined` and `''` all count as missing. */
export function orUnknown(value: string | null | undefined): string {
  return value || UNKNOWN_VALUE;
}
