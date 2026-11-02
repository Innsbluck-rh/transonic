//! Windows taskbar thumbnail toolbar: previous / play-pause / next buttons
//! under the window's thumbnail in the taskbar button flyout.
//!
//! ## Layering
//!
//! Same shape as SMTC (`smtc_windows.rs`):
//! - **State → OS** rides the [`PlaybackReporter`](super::reporting) fan-out:
//!   [`ThumbBarReporter`] is one more reporter, wired in at the composition
//!   root (`create_playback_controller`) behind `#[cfg(windows)]`.
//! - **OS → app** (button clicks) routes back through the ordinary
//!   `#[tauri::command]` playback entry points, so a click behaves exactly like
//!   a player bar button press.
//!
//! The platform-independent parts (button state, icon pixels) live in
//! [`super::thumbbar`].
//!
//! ## Win32 lifecycle
//!
//! - `ITaskbarList3` may only be used once the window has received the
//!   registered `TaskbarButtonCreated` message. Explorer sends it again after
//!   it restarts, for a new taskbar button that needs the toolbar added again.
//! - Tauri creates the (visible) main window before running `setup`, so the
//!   first `TaskbarButtonCreated` is already delivered to tao's window
//!   procedure by the time [`init`] subclasses the window. [`init`] therefore
//!   adds the toolbar right away; if the button doesn't exist yet, that fails
//!   and the later message adds it.
//! - `ThumbBarAddButtons` fixes the set and order of the buttons; later
//!   changes (glyph, enabled state) go through `ThumbBarUpdateButtons`.
//! - Clicks arrive as `WM_COMMAND` with HIWORD = `THBN_CLICKED` and LOWORD =
//!   the button id, so the window is subclassed (`SetWindowSubclass`, which tao
//!   also uses for its own window procedure) to receive them.
//! - Glyphs are drawn black when the system ("Windows mode") theme is light and
//!   white otherwise, and are re-rendered on theme and DPI changes.
//!
//! ## Threading
//!
//! The subclass procedure runs on the main thread, which owns the window. The
//! `ITaskbarList3` and the toolbar state live in a main-thread thread-local
//! ([`THUMBBAR`]); `report_state` is called from background threads and
//! marshals changes via [`AppHandle::run_on_main_thread`]. COM calls into the
//! shell can pump messages and re-enter the subclass procedure, so no borrow of
//! [`THUMBBAR`] is held across them.

use std::cell::RefCell;

use tauri::{AppHandle, Manager};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{
    DefSubclassProc, ITaskbarList3, SetWindowSubclass, TaskbarList, THBF_DISABLED, THBF_ENABLED,
    THBN_CLICKED, THB_FLAGS, THB_ICON, THB_TOOLTIP, THUMBBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIcon, DestroyIcon, RegisterWindowMessageW, HICON, SM_CXICON, WM_COMMAND, WM_DPICHANGED,
    WM_SETTINGCHANGE,
};

use super::reporting::PlaybackReporter;
use super::thumbbar::{
    self, ButtonState, Glyph, BUTTON_ID_NEXT, BUTTON_ID_PLAY_PAUSE, BUTTON_ID_PREV,
};
use crate::models::{PlaybackStatus, PlayingState};

/// `uIdSubclass` of our window subclass; only needs to be unique per window.
const SUBCLASS_ID: usize = 0x7468_6d62;

thread_local! {
    /// The toolbar's COM object and state, pinned to the main/UI thread. Kept
    /// for the process lifetime, like the subclass itself.
    static THUMBBAR: RefCell<Option<ThumbBar>> = const { RefCell::new(None) };
}

struct ThumbBar {
    app: AppHandle,
    hwnd: HWND,
    taskbar: ITaskbarList3,
    /// Message id of the registered `TaskbarButtonCreated` message.
    taskbar_button_created: u32,
    /// State to show; also resolves a play/pause click.
    state: ButtonState,
    /// Whether the current taskbar button has the toolbar yet.
    added: bool,
}

/// A [`PlaybackReporter`] that mirrors playback state into the thumbnail
/// toolbar.
pub struct ThumbBarReporter {
    app: AppHandle,
    /// State last sent to the toolbar, for dropping position-only ticks.
    last_state: Option<ButtonState>,
}

/// Subclass the main window and return a reporter to wire into the composite.
///
/// Best-effort: any failure is logged and `None` is returned so the app runs
/// normally without the toolbar. Must be called on the main thread (it is,
/// from `create_playback_controller` during Tauri `setup`).
pub fn init(app: &AppHandle) -> Option<ThumbBarReporter> {
    let window = match app.get_webview_window("main") {
        Some(window) => window,
        None => {
            log::error!("thumbbar: main window not found; thumbnail toolbar disabled");
            return None;
        }
    };
    let hwnd = match window.hwnd() {
        Ok(handle) => HWND(handle.0),
        Err(error) => {
            log::error!("thumbbar: failed to obtain HWND: {error}; thumbnail toolbar disabled");
            return None;
        }
    };

    let taskbar_button_created = unsafe { RegisterWindowMessageW(w!("TaskbarButtonCreated")) };
    if taskbar_button_created == 0 {
        log::error!("thumbbar: RegisterWindowMessageW failed; thumbnail toolbar disabled");
        return None;
    }
    let taskbar: ITaskbarList3 =
        match unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER) } {
            Ok(taskbar) => taskbar,
            Err(error) => {
                log::error!(
                    "thumbbar: creating ITaskbarList3 failed: {error}; thumbnail toolbar disabled"
                );
                return None;
            }
        };
    if let Err(error) = unsafe { taskbar.HrInit() } {
        log::error!("thumbbar: ITaskbarList3::HrInit failed: {error}; thumbnail toolbar disabled");
        return None;
    }

    // Stored before subclassing so the procedure finds it from the first message.
    THUMBBAR.with(|slot| {
        *slot.borrow_mut() = Some(ThumbBar {
            app: app.clone(),
            hwnd,
            taskbar,
            taskbar_button_created,
            state: ButtonState::from_playing_state(&PlayingState::Idle),
            added: false,
        })
    });
    if !unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0) }.as_bool() {
        THUMBBAR.with(|slot| *slot.borrow_mut() = None);
        log::error!("thumbbar: SetWindowSubclass failed; thumbnail toolbar disabled");
        return None;
    }
    log::info!("thumbbar: initialized");
    // The first `TaskbarButtonCreated` has normally been delivered already
    // (see the module docs), so don't wait for it.
    apply();

    Some(ThumbBarReporter {
        app: app.clone(),
        last_state: None,
    })
}

impl PlaybackReporter for ThumbBarReporter {
    fn report_state(&mut self, status: &PlaybackStatus) -> Result<(), String> {
        let state = ButtonState::from_playing_state(&status.playing_state);
        if self.last_state == Some(state) {
            return Ok(());
        }
        self.last_state = Some(state);

        if let Err(error) = self.app.run_on_main_thread(move || {
            let added = with_thumbbar(|bar| {
                bar.state = state;
                bar.added
            });
            // Before `TaskbarButtonCreated` the state is only stored; adding
            // the toolbar then picks it up.
            if added == Some(true) {
                apply();
            }
        }) {
            log::warn!("thumbbar: run_on_main_thread failed: {error}");
        }
        Ok(())
    }
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    _ref_data: usize,
) -> LRESULT {
    let Some((taskbar_button_created, added)) =
        with_thumbbar(|bar| (bar.taskbar_button_created, bar.added))
    else {
        return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
    };

    if msg == taskbar_button_created {
        // Also sent after an Explorer restart, for a button without our toolbar.
        log::info!("thumbbar: TaskbarButtonCreated received");
        with_thumbbar(|bar| bar.added = false);
        apply();
    } else if msg == WM_COMMAND && hiword(wparam) == THBN_CLICKED {
        handle_click(loword(wparam));
        return LRESULT(0);
    } else if added
        && (msg == WM_DPICHANGED || (msg == WM_SETTINGCHANGE && is_immersive_color_set(lparam)))
    {
        apply();
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// Map a button click back onto the ordinary playback commands.
fn handle_click(button_id: u32) {
    use crate::commands::{playback_next, playback_pause, playback_play, playback_prev};

    let Some((app, playing)) = with_thumbbar(|bar| (bar.app.clone(), bar.state.playing)) else {
        return;
    };
    match button_id {
        BUTTON_ID_PREV => {
            let _ = playback_prev(app);
        }
        BUTTON_ID_PLAY_PAUSE => {
            // Resolve against the glyph the button shows.
            let _ = if playing {
                playback_pause(app)
            } else {
                playback_play(app)
            };
        }
        BUTTON_ID_NEXT => {
            let _ = playback_next(app);
        }
        other => log::debug!("thumbbar: click on unknown button id {other}"),
    }
}

/// Push the current state to the toolbar: add it if the current taskbar button
/// has none yet, otherwise update it. Runs on the main thread.
fn apply() {
    let Some((taskbar, hwnd, state, added)) =
        with_thumbbar(|bar| (bar.taskbar.clone(), bar.hwnd, bar.state, bar.added))
    else {
        return;
    };

    let size = icon_size(hwnd);
    let rgb = glyph_rgb();
    let flags = if state.enabled {
        THBF_ENABLED
    } else {
        THBF_DISABLED
    };
    let mut icons = Vec::with_capacity(3);
    let mut buttons = Vec::with_capacity(3);
    for (id, glyph) in thumbbar::buttons(state) {
        let icon = match create_icon(glyph, size, rgb) {
            Ok(icon) => icon,
            Err(error) => {
                log::warn!("thumbbar: {error}");
                return;
            }
        };
        buttons.push(THUMBBUTTON {
            dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
            iId: id,
            hIcon: icon.0,
            szTip: tooltip(glyph.tooltip()),
            dwFlags: flags,
            ..Default::default()
        });
        icons.push(icon);
    }

    // The taskbar copies the icons, so `icons` can be dropped after the call.
    if added {
        if let Err(error) = unsafe { taskbar.ThumbBarUpdateButtons(hwnd, &buttons) } {
            log::warn!("thumbbar: failed to update toolbar buttons: {error}");
        }
        return;
    }
    match unsafe { taskbar.ThumbBarAddButtons(hwnd, &buttons) } {
        Ok(()) => {
            with_thumbbar(|bar| bar.added = true);
            log::info!("thumbbar: toolbar added");
        }
        // Adding fails both before the taskbar button exists and when this
        // button already has the toolbar (added by `init`, with the button's
        // `TaskbarButtonCreated` delivered only afterwards). Updating succeeds
        // only in the latter case.
        Err(add_error) => match unsafe { taskbar.ThumbBarUpdateButtons(hwnd, &buttons) } {
            Ok(()) => {
                with_thumbbar(|bar| bar.added = true);
                log::info!("thumbbar: toolbar already present; updated");
            }
            Err(_) => log::info!("thumbbar: toolbar not added yet: {add_error}"),
        },
    }
}

/// Run `f` on the toolbar state, or return `None` if it isn't initialized or
/// is already borrowed further up the stack.
fn with_thumbbar<R>(f: impl FnOnce(&mut ThumbBar) -> R) -> Option<R> {
    THUMBBAR.with(|slot| {
        let mut guard = slot.try_borrow_mut().ok()?;
        guard.as_mut().map(f)
    })
}

/// An `HICON` destroyed on drop.
struct OwnedIcon(HICON);

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        let _ = unsafe { DestroyIcon(self.0) };
    }
}

fn create_icon(glyph: Glyph, size: u32, rgb: [u8; 3]) -> Result<OwnedIcon, String> {
    let bgra = thumbbar::glyph_bgra(glyph, size, rgb)?;
    let mask = thumbbar::and_mask(&bgra, size);
    unsafe {
        CreateIcon(
            None,
            size as i32,
            size as i32,
            1,
            32,
            mask.as_ptr(),
            bgra.as_ptr(),
        )
    }
    .map(OwnedIcon)
    .map_err(|error| format!("CreateIcon for {glyph:?} failed: {error}"))
}

/// Button image size the toolbar expects: `SM_CXICON` at the window's DPI.
fn icon_size(hwnd: HWND) -> u32 {
    let dpi = match unsafe { GetDpiForWindow(hwnd) } {
        0 => 96,
        dpi => dpi,
    };
    let size = unsafe { GetSystemMetricsForDpi(SM_CXICON, dpi) };
    u32::try_from(size)
        .ok()
        .filter(|size| *size > 0)
        .unwrap_or(32)
}

/// Glyph color for the system ("Windows mode") theme: black when it is light,
/// white otherwise (including when the value can't be read).
fn glyph_rgb() -> [u8; 3] {
    let mut value: u32 = 0;
    let mut value_size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut value_size),
        )
    };
    if status == ERROR_SUCCESS && value == 1 {
        [0, 0, 0]
    } else {
        [255, 255, 255]
    }
}

/// `WM_SETTINGCHANGE` with `"ImmersiveColorSet"` is the light/dark theme switch.
fn is_immersive_color_set(lparam: LPARAM) -> bool {
    if lparam.0 == 0 {
        return false;
    }
    unsafe { PCWSTR(lparam.0 as *const u16).to_string() }
        .is_ok_and(|setting| setting == "ImmersiveColorSet")
}

fn tooltip(text: &str) -> [u16; 260] {
    let mut buffer = [0u16; 260];
    // Leave the last unit as the terminating NUL.
    for (slot, unit) in buffer.iter_mut().zip(text.encode_utf16().take(259)) {
        *slot = unit;
    }
    buffer
}

fn loword(wparam: WPARAM) -> u32 {
    (wparam.0 & 0xffff) as u32
}

fn hiword(wparam: WPARAM) -> u32 {
    ((wparam.0 >> 16) & 0xffff) as u32
}
