//! Placeholder shown when a tag that names the item (song title, artist, album
//! name) has no value — the rule in `docs/unavailable-value-fallback.md`.
//!
//! Browse commands pass missing values through as `None` / `""`; the
//! placeholder is applied only where Rust itself displays a value (SMTC, the
//! Android media session). The frontend counterpart is
//! `src/utils/unknownValue.ts`; keep the two strings identical.

pub(crate) const UNKNOWN_VALUE: &str = "[unknown]";

/// The value, or [`UNKNOWN_VALUE`] when it is missing. `None` and an empty
/// string both count as missing.
pub(crate) fn or_unknown(value: Option<&str>) -> String {
    match value {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => UNKNOWN_VALUE.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{or_unknown, UNKNOWN_VALUE};

    #[test]
    fn missing_and_empty_values_use_the_placeholder() {
        assert_eq!(or_unknown(None), UNKNOWN_VALUE);
        assert_eq!(or_unknown(Some("")), UNKNOWN_VALUE);
        assert_eq!(or_unknown(Some("Song")), "Song");
    }
}
