//! Platform-independent parts of the Windows taskbar thumbnail toolbar
//! (`thumbbar_windows.rs`): which buttons to show for a playback state, and
//! the pixels of their icons.
//!
//! Kept apart from the Win32 glue so it can be unit-tested on any host; the
//! glue itself is excluded from test builds, like SMTC.

use image::{imageops::FilterType, ImageFormat};

use crate::models::PlayingState;

/// Button ids, delivered back as the LOWORD of `WM_COMMAND` on a click.
pub const BUTTON_ID_PREV: u32 = 1;
pub const BUTTON_ID_PLAY_PAUSE: u32 = 2;
pub const BUTTON_ID_NEXT: u32 = 3;

/// What the toolbar shows: the play/pause glyph and whether the buttons accept
/// clicks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonState {
    pub playing: bool,
    pub enabled: bool,
}

impl ButtonState {
    pub fn from_playing_state(state: &PlayingState) -> Self {
        Self {
            playing: matches!(state, PlayingState::Playing),
            // Same rule as the player bar (`isControlDisabledForState` in
            // `src/features/playback/usePlayback.ts`).
            enabled: !matches!(state, PlayingState::Idle | PlayingState::Interrupted),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Prev,
    Play,
    Pause,
    Next,
}

impl Glyph {
    /// Tooltip shown when hovering the button.
    pub fn tooltip(self) -> &'static str {
        match self {
            Glyph::Prev => "Previous",
            Glyph::Play => "Play",
            Glyph::Pause => "Pause",
            Glyph::Next => "Next",
        }
    }

    /// 128x128 PNGs of the player bar's Material Symbols glyphs
    /// (`@iconify-json/material-symbols` 1.2.97: `skip-previous`, `play-arrow`,
    /// `pause`, `skip-next`), each wrapped in a `viewBox="0 0 24 24"` SVG and
    /// rendered with `magick -background none -density 512 <name>.svg
    /// PNG32:<name>.png`. Only the alpha channel is used; the color is applied
    /// at load time.
    fn png(self) -> &'static [u8] {
        match self {
            Glyph::Prev => include_bytes!("../../icons/thumbbar/prev.png"),
            Glyph::Play => include_bytes!("../../icons/thumbbar/play.png"),
            Glyph::Pause => include_bytes!("../../icons/thumbbar/pause.png"),
            Glyph::Next => include_bytes!("../../icons/thumbbar/next.png"),
        }
    }
}

/// The toolbar's buttons, left to right. The set and order are fixed once the
/// toolbar is added; only the play/pause glyph changes with `state`.
pub fn buttons(state: ButtonState) -> [(u32, Glyph); 3] {
    let play_pause = if state.playing {
        Glyph::Pause
    } else {
        Glyph::Play
    };
    [
        (BUTTON_ID_PREV, Glyph::Prev),
        (BUTTON_ID_PLAY_PAUSE, play_pause),
        (BUTTON_ID_NEXT, Glyph::Next),
    ]
}

/// Render `glyph` as a `size_px` square filled with `rgb`: top-down rows of
/// straight-alpha BGRA, the layout of a 32bpp top-down DIB.
pub fn glyph_bgra(glyph: Glyph, size_px: u32, rgb: [u8; 3]) -> Result<Vec<u8>, String> {
    let mut image = image::load_from_memory_with_format(glyph.png(), ImageFormat::Png)
        .map_err(|error| format!("failed to decode {glyph:?} glyph: {error}"))?
        .into_rgba8();
    if image.dimensions() != (size_px, size_px) {
        image = image::imageops::resize(&image, size_px, size_px, FilterType::Lanczos3);
    }

    let [red, green, blue] = rgb;
    let mut bgra = Vec::with_capacity(image.as_raw().len());
    for pixel in image.pixels() {
        bgra.extend_from_slice(&[blue, green, red, pixel.0[3]]);
    }
    Ok(bgra)
}

/// The 1bpp AND mask `CreateIcon` takes alongside 32bpp color bits: a set bit
/// marks a fully transparent pixel. Rows are padded to 16 bits.
pub fn and_mask(bgra: &[u8], size_px: u32) -> Vec<u8> {
    let size = size_px as usize;
    let stride = size.div_ceil(16) * 2;
    let mut mask = vec![0u8; stride * size];
    for (index, pixel) in bgra.chunks_exact(4).enumerate() {
        if pixel[3] == 0 {
            let (y, x) = (index / size, index % size);
            mask[y * stride + x / 8] |= 0x80 >> (x % 8);
        }
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::{
        and_mask, buttons, glyph_bgra, ButtonState, Glyph, BUTTON_ID_NEXT, BUTTON_ID_PLAY_PAUSE,
        BUTTON_ID_PREV,
    };
    use crate::models::PlayingState;

    #[test]
    fn button_state_follows_player_bar_rules() {
        let cases = [
            (PlayingState::Idle, false, false),
            (PlayingState::Playing, true, true),
            (PlayingState::Paused, false, true),
            (PlayingState::Stopped, false, true),
            (PlayingState::Interrupted, false, false),
            (PlayingState::Error, false, true),
        ];
        for (state, playing, enabled) in cases {
            assert_eq!(
                ButtonState::from_playing_state(&state),
                ButtonState { playing, enabled },
                "{state:?}"
            );
        }
    }

    #[test]
    fn play_pause_button_shows_pause_while_playing() {
        let playing = ButtonState {
            playing: true,
            enabled: true,
        };
        let paused = ButtonState {
            playing: false,
            enabled: true,
        };
        assert_eq!(
            buttons(playing),
            [
                (BUTTON_ID_PREV, Glyph::Prev),
                (BUTTON_ID_PLAY_PAUSE, Glyph::Pause),
                (BUTTON_ID_NEXT, Glyph::Next),
            ]
        );
        assert_eq!(buttons(paused)[1], (BUTTON_ID_PLAY_PAUSE, Glyph::Play));
    }

    #[test]
    fn glyph_bgra_has_one_bgra_pixel_per_square_pixel() {
        for glyph in [Glyph::Prev, Glyph::Play, Glyph::Pause, Glyph::Next] {
            for size in [32, 40, 48, 128] {
                let bgra = glyph_bgra(glyph, size, [255, 255, 255]).unwrap();
                assert_eq!(bgra.len(), (size * size * 4) as usize, "{glyph:?} at {size}px");
            }
        }
    }

    #[test]
    fn glyph_bgra_replaces_color_and_keeps_alpha() {
        let bgra = glyph_bgra(Glyph::Play, 128, [10, 20, 30]).unwrap();
        let source = image::load_from_memory(Glyph::Play.png())
            .unwrap()
            .into_rgba8();

        for (out, src) in bgra.chunks_exact(4).zip(source.pixels()) {
            assert_eq!(out, &[30, 20, 10, src.0[3]]);
        }
        // The triangle covers the center and leaves the corners empty.
        let alpha_at = |x: usize, y: usize| bgra[(y * 128 + x) * 4 + 3];
        assert_eq!(alpha_at(64, 64), 255);
        assert_eq!(alpha_at(0, 0), 0);
        assert_eq!(alpha_at(127, 127), 0);
    }

    #[test]
    fn and_mask_marks_transparent_pixels_in_word_aligned_rows() {
        // 20px wide: 3 bytes of bits padded to a 4-byte row.
        let size = 20;
        let mut bgra = vec![255u8; size * size * 4];
        let transparent = |bgra: &mut Vec<u8>, x: usize, y: usize| bgra[(y * size + x) * 4 + 3] = 0;
        transparent(&mut bgra, 0, 0);
        transparent(&mut bgra, 19, 1);

        let mask = and_mask(&bgra, size as u32);

        assert_eq!(mask.len(), 4 * size);
        let mut expected = vec![0u8; 4 * size];
        expected[0] = 0x80;
        expected[4 + 2] = 0x10;
        assert_eq!(mask, expected);
    }
}
