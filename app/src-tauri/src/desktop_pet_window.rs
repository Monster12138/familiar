use familiar_core::config::DesktopPetConfig;
use std::sync::Mutex;

#[cfg(target_os = "macos")]
use tauri::Manager;
#[cfg(target_os = "macos")]
use tauri_nspanel::{tauri_panel, PanelLevel, StyleMask, WebviewWindowExt};

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(DesktopPetPanel {
        config: {
            can_become_key_window: true,
            can_become_main_window: false,
            hides_on_deactivate: false,
            is_floating_panel: true
        }
    })
}

/// A regular Tauri NSWindow can still disappear behind macOS full-screen Spaces
/// even with `visibleOnAllWorkspaces`. Keep only the pet itself as an NSPanel;
/// settings and other application windows remain normal NSWindows.
pub fn initialize(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let panel = window
            .to_panel::<DesktopPetPanel>()
            .map_err(|error| error.to_string())?;

        panel.set_level(PanelLevel::Floating.value());
        panel.set_style_mask(StyleMask::empty().nonactivating_panel().into());
        panel.set_hides_on_deactivate(false);
        tracing::info!("converted desktop pet window to NSPanel");
    }

    #[cfg(not(target_os = "macos"))]
    let _ = window;

    Ok(())
}

pub fn apply_settings(
    window: &tauri::WebviewWindow,
    config: &DesktopPetConfig,
    restore_position: bool,
) -> Result<(), String> {
    window
        .set_always_on_top(config.always_on_top)
        .map_err(|error| error.to_string())?;
    window
        .set_visible_on_all_workspaces(config.show_on_all_desktops)
        .map_err(|error| error.to_string())?;
    // Click-through: on macOS the click_through module is the SOLE owner of
    // ignoresMouseEvents (toggling it from anywhere else races its main-thread
    // apply and leaves the window live after enabling). Other platforms fall
    // back to ignoring all cursor events.
    #[cfg(target_os = "macos")]
    {
        click_through::install(window)?;
        click_through::set_enabled(config.click_through);
        hover::install(window)?;
    }
    #[cfg(not(target_os = "macos"))]
    window
        .set_ignore_cursor_events(config.click_through)
        .map_err(|error| error.to_string())?;

    // Windows hover detection must keep working while click-through is on,
    // where the webview never sees pointer events (see the `hover` module).
    #[cfg(target_os = "windows")]
    hover::install(window)?;

    // Position is owned by the drag-save handler, the startup restore and the
    // off-screen watchdog. Only restore it on startup; applying it on every
    // settings change would yank the window back to a possibly stale position
    // (e.g. toggling click-through must not move the pet).
    if restore_position {
        let spec = parse_position(&config.position);
        let resolved = resolve_position(window, spec);
        let _ = window.set_position(tauri::Position::Physical(resolved));
    }

    #[cfg(target_os = "macos")]
    apply_macos_desktop_behavior(window, config.show_on_all_desktops)?;

    Ok(())
}

/// Toggle left-click pass-through at runtime. The right-click menu uses this
/// to temporarily disable pass-through while it is open, so its items remain
/// clickable; the caller restores the configured value when the menu closes.
pub fn set_click_through_enabled(enabled: bool) {
    #[cfg(target_os = "macos")]
    click_through::set_enabled(enabled);
    #[cfg(not(target_os = "macos"))]
    let _ = enabled;
}

/// Resize the pet window, optionally keeping its bottom edge and horizontal
/// center fixed. Bubble content grows above the pet, so anchored resizes
/// prevent it from pushing the pet down on screen.
pub fn resize(
    window: &tauri::WebviewWindow,
    width: f64,
    height: f64,
    anchor_bottom: bool,
) -> Result<(), String> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err("window dimensions must be positive finite numbers".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let window_for_main_thread = window.clone();
        window
            .run_on_main_thread(move || {
                use cocoa::appkit::NSWindow;
                use cocoa::base::{id, YES};
                use cocoa::foundation::{NSPoint, NSRect, NSSize};

                let Ok(ns_window_ptr) = window_for_main_thread.ns_window() else {
                    tracing::warn!("failed to access the desktop pet NSPanel for resize");
                    return;
                };
                let ns_window = ns_window_ptr as id;

                unsafe {
                    let frame = ns_window.frame();
                    let x = if anchor_bottom {
                        frame.origin.x + (frame.size.width - width) / 2.0
                    } else {
                        frame.origin.x
                    };
                    let y = if anchor_bottom {
                        frame.origin.y
                    } else {
                        frame.origin.y + frame.size.height - height
                    };
                    let resized_frame = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));
                    ns_window.setFrame_display_(resized_frame, YES);
                }
            })
            .map_err(|error| error.to_string())
    }

    #[cfg(not(target_os = "macos"))]
    {
        if !anchor_bottom {
            return window
                .set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)))
                .map_err(|error| error.to_string());
        }

        let old_size = window.outer_size().map_err(|error| error.to_string())?;
        let old_position = window.outer_position().map_err(|error| error.to_string())?;
        let scale_factor = window.scale_factor().map_err(|error| error.to_string())?;
        let new_width = (width * scale_factor).round() as i32;
        let new_height = (height * scale_factor).round() as i32;
        let x = old_position.x - (new_width - old_size.width as i32) / 2;
        let y = old_position.y - (new_height - old_size.height as i32);

        window
            .set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)))
            .map_err(|error| error.to_string())?;
        window
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
                x, y,
            )))
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

/// Logical-pixel geometry for front-end context-menu placement: the window's
/// top-left position, its size, and the active monitor's work area.
#[derive(serde::Serialize)]
pub struct MenuGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub work_left: f64,
    pub work_top: f64,
    pub work_right: f64,
    pub work_bottom: f64,
}

pub fn menu_geometry(window: &tauri::WebviewWindow) -> Result<MenuGeometry, String> {
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let pos = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| "no monitor available".to_string())?;
    let work = monitor.work_area();
    Ok(MenuGeometry {
        x: pos.x as f64 / scale,
        y: pos.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
        work_left: work.position.x as f64 / scale,
        work_top: work.position.y as f64 / scale,
        work_right: (work.position.x + work.size.width as i32) as f64 / scale,
        work_bottom: (work.position.y + work.size.height as i32) as f64 / scale,
    })
}

/// Move + resize the pet window as one frame, in logical top-left screen
/// coordinates. The context menu uses this to grow the window in any
/// direction (the front-end offsets the pet container by the origin delta so
/// the pet itself never moves).
pub fn set_frame(
    window: &tauri::WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if !x.is_finite()
        || !y.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
    {
        return Err("window frame must be finite with positive size".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let scale = window.scale_factor().map_err(|error| error.to_string())?;
        let pos = window.outer_position().map_err(|error| error.to_string())?;
        let x0 = pos.x as f64 / scale;
        let y0 = pos.y as f64 / scale;
        let window_for_main_thread = window.clone();
        window
            .run_on_main_thread(move || {
                use cocoa::appkit::NSWindow;
                use cocoa::base::{id, YES};
                use cocoa::foundation::{NSPoint, NSRect, NSSize};

                let Ok(ns_window_ptr) = window_for_main_thread.ns_window() else {
                    tracing::warn!("failed to access the desktop pet NSPanel for set_frame");
                    return;
                };
                let ns_window = ns_window_ptr as id;
                unsafe {
                    let frame = ns_window.frame();
                    // Translate the requested top-left screen coordinates into
                    // Cocoa's bottom-left origin using the current frame.
                    let nx = frame.origin.x + (x - x0);
                    let ny = frame.origin.y + (frame.size.height - height) - (y - y0);
                    let new_frame = NSRect::new(NSPoint::new(nx, ny), NSSize::new(width, height));
                    ns_window.setFrame_display_(new_frame, YES);
                }
            })
            .map_err(|error| error.to_string())
    }

    #[cfg(not(target_os = "macos"))]
    {
        window
            .set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)))
            .map_err(|error| error.to_string())?;
        window
            .set_size(tauri::Size::Logical(tauri::LogicalSize::new(width, height)))
            .map_err(|error| error.to_string())
    }
}

// --- Event-driven hover detection ------------------------------------------
//
// A non-activating floating panel never becomes the active window, and macOS
// only delivers mouse-moved events to the active window — so neither CSS :hover
// nor an NSTrackingArea fires until the user clicks the pet. To detect hover
// without activation we register system-wide mouse-moved monitors (global for
// events going to other apps, local for the rare events going to us). These are
// pure event callbacks — no polling: they run only when the mouse actually
// moves, then we check whether the cursor is over the pet window and tell the
// front-end, which dims the content container (keeping the context menu opaque).
#[cfg(target_os = "macos")]
mod hover {
    use cocoa::base::id;
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
    use std::sync::OnceLock;
    use tauri::{Emitter, Manager};

    static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
    static PANEL: AtomicPtr<Object> = AtomicPtr::new(std::ptr::null_mut());
    static LAST_HOVER: AtomicBool = AtomicBool::new(false);
    // Keep the monitor objects alive for the process lifetime.
    static GLOBAL_MONITOR: AtomicPtr<Object> = AtomicPtr::new(std::ptr::null_mut());
    static LOCAL_MONITOR: AtomicPtr<Object> = AtomicPtr::new(std::ptr::null_mut());

    /// Compare the cursor against the pet window frame and emit a hover change.
    fn handle_mouse_event() {
        use cocoa::foundation::{NSPoint, NSRect};
        let panel = PANEL.load(Ordering::SeqCst);
        if panel.is_null() {
            return;
        }
        unsafe {
            // Both mouseLocation and frame are in global screen coordinates with
            // a bottom-left origin, so they compare directly across monitors.
            let mouse: NSPoint = msg_send![class!(NSEvent), mouseLocation];
            let frame: NSRect = msg_send![panel, frame];
            let inside = mouse.x >= frame.origin.x
                && mouse.x <= frame.origin.x + frame.size.width
                && mouse.y >= frame.origin.y
                && mouse.y <= frame.origin.y + frame.size.height;
            let last = LAST_HOVER.load(Ordering::SeqCst);
            if inside != last {
                LAST_HOVER.store(inside, Ordering::SeqCst);
                if let Some(app) = APP.get() {
                    let _ = app.emit("pet_hover_changed", inside);
                }
            }
        }
    }

    /// Install system-wide mouse-moved monitors so hover works without the panel
    /// ever being activated. Idempotent.
    pub fn install(window: &tauri::WebviewWindow) -> Result<(), String> {
        let _ = APP.set(window.app_handle().clone());
        let window_for_main = window.clone();
        window
            .run_on_main_thread(move || {
                static ONCE: std::sync::Once = std::sync::Once::new();
                ONCE.call_once(|| {
                    unsafe {
                        let Ok(ns_window_ptr) = window_for_main.ns_window() else {
                            tracing::warn!("hover: no NSWindow available");
                            return;
                        };
                        let ns_window = ns_window_ptr as id;
                        PANEL.store(ns_window, Ordering::SeqCst);

                        // NSEventMaskMouseMoved = 1 << NSEventTypeMouseMoved(5).
                        let mask: u64 = 1 << 5;

                        // Global monitor: mouse moves delivered to other apps
                        // (the common case, since the pet floats above them).
                        let global_block = block::ConcreteBlock::new(|_event: id| {
                            handle_mouse_event();
                        })
                        .copy();
                        let global: id = msg_send![
                            class!(NSEvent),
                            addGlobalMonitorForEventsMatchingMask: mask
                            handler: &*global_block
                        ];
                        GLOBAL_MONITOR.store(global, Ordering::SeqCst);

                        // Local monitor: the rare moves delivered to our own app.
                        // It must return the event unchanged.
                        let local_block = block::ConcreteBlock::new(|event: id| -> id {
                            handle_mouse_event();
                            event
                        })
                        .copy();
                        let local: id = msg_send![
                            class!(NSEvent),
                            addLocalMonitorForEventsMatchingMask: mask
                            handler: &*local_block
                        ];
                        LOCAL_MONITOR.store(local, Ordering::SeqCst);

                        tracing::info!("hover: mouse-move monitors installed");
                    }
                });
            })
            .map_err(|error| error.to_string())
    }
}

// --- Windows mouse monitoring via a low-level mouse hook -------------------
//
// The pet window can be hit-test transparent (click-through), which starves
// the webview of mouse-move events, so DOM :hover is unreliable there. macOS
// solves this with NSEvent monitors; Windows has no equivalent global API, so
// we use a WH_MOUSE_LL low-level mouse hook instead. It observes every mouse
// move on the desktop (a pure event callback — no polling), compares the
// cursor against the pet window rect and emits `pet_hover_changed` on state
// changes, matching the macOS module's contract. The same hook also tracks the
// physical left-button state so the drag handler in main.rs can tell real
// drags apart from spurious Moved events.
#[cfg(target_os = "windows")]
pub(crate) mod hover {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
    use std::sync::OnceLock;
    use tauri::{Emitter, Manager};

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, GetWindowRect, SetWindowsHookExW,
        TranslateMessage, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_MOUSEMOVE, WM_NCMOUSEMOVE,
    };

    static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
    static PANEL_HWND: AtomicIsize = AtomicIsize::new(0);
    static LAST_HOVER: AtomicBool = AtomicBool::new(false);
    /// Physical left-button state, sampled by the hook.
    static LEFT_BUTTON_DOWN: AtomicBool = AtomicBool::new(false);
    /// Keeps the hook alive for the process lifetime.
    static HOOK: AtomicIsize = AtomicIsize::new(0);

    /// Whether the physical left mouse button is currently held.
    pub(crate) fn left_button_down() -> bool {
        LEFT_BUTTON_DOWN.load(Ordering::Relaxed)
    }

    /// Whether a physical screen point lies within a physical screen rect
    /// (inclusive edges).
    fn point_in_rect(point: POINT, rect: RECT) -> bool {
        point.x >= rect.left
            && point.x <= rect.right
            && point.y >= rect.top
            && point.y <= rect.bottom
    }

    /// Whether the cursor currently rests over the pet window rect.
    fn cursor_over_window(cursor: POINT) -> bool {
        let hwnd = HWND(PANEL_HWND.load(Ordering::SeqCst) as *mut c_void);
        if hwnd.0.is_null() {
            return false;
        }
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_err() {
                return false;
            }
            // Both the hook point and the window rect are in physical screen
            // coordinates, so they compare directly across monitors.
            point_in_rect(cursor, rect)
        }
    }

    /// WH_MOUSE_LL callback; runs on the hook thread and must stay cheap and
    /// non-blocking.
    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            match wparam.0 as u32 {
                // During a caption drag the cursor counts as being over the
                // non-client area, so both move message kinds reach the hook.
                // In a WH_MOUSE_LL callback wParam is the message identifier,
                // not the message's MK_* button bitmask. MSLLHOOKSTRUCT.flags
                // contains LLMHF_* injection flags, so query the actual async
                // key state instead.
                WM_MOUSEMOVE | WM_NCMOUSEMOVE => {
                    let button_down = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 };
                    LEFT_BUTTON_DOWN.store(button_down, Ordering::Relaxed);
                    let inside = cursor_over_window(info.pt);
                    let last = LAST_HOVER.swap(inside, Ordering::SeqCst);
                    if inside != last {
                        if let Some(app) = APP.get() {
                            let _ = app.emit("pet_hover_changed", inside);
                        }
                    }
                }
                // A fresh press starts a clean drag: drop any escape latch
                // left over from an earlier drag that ended without a final
                // Moved event (where the main-thread reset never ran).
                WM_LBUTTONDOWN => {
                    LEFT_BUTTON_DOWN.store(true, Ordering::Relaxed);
                    super::reset_drag_escape();
                }
                WM_LBUTTONUP => LEFT_BUTTON_DOWN.store(false, Ordering::Relaxed),
                _ => {}
            }
        }
        // Every event must continue down the hook chain.
        CallNextHookEx(None, code, wparam, lparam)
    }

    /// Install the low-level mouse hook on a dedicated message-loop thread.
    /// Idempotent; the window handle is refreshed on every call in case the
    /// pet window is ever recreated.
    pub fn install(window: &tauri::WebviewWindow) -> Result<(), String> {
        let _ = APP.set(window.app_handle().clone());
        let hwnd = window.hwnd().map_err(|error| error.to_string())?;
        PANEL_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let builder = std::thread::Builder::new().name("familiar-hover-hook".to_string());
            if let Err(error) = builder.spawn(move || unsafe {
                let Ok(hook) = SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), None, 0) else {
                    tracing::error!("hover: failed to install low-level mouse hook");
                    return;
                };
                HOOK.store(hook.0 as isize, Ordering::SeqCst);
                tracing::info!("hover: low-level mouse hook installed");

                // A low-level hook fires only while the installing thread pumps
                // messages; run the loop until the process exits.
                let mut message = MSG::default();
                while GetMessageW(&mut message, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }) {
                tracing::error!("hover: failed to spawn hook thread: {error}");
            }
        });
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn point_in_rect_is_inclusive_on_edges() {
            let rect = RECT {
                left: 10,
                top: 20,
                right: 110,
                bottom: 120,
            };
            assert!(point_in_rect(POINT { x: 10, y: 20 }, rect));
            assert!(point_in_rect(POINT { x: 110, y: 120 }, rect));
            assert!(point_in_rect(POINT { x: 60, y: 70 }, rect));
        }

        #[test]
        fn point_in_rect_rejects_outside_points() {
            let rect = RECT {
                left: 10,
                top: 20,
                right: 110,
                bottom: 120,
            };
            assert!(!point_in_rect(POINT { x: 9, y: 70 }, rect));
            assert!(!point_in_rect(POINT { x: 111, y: 70 }, rect));
            assert!(!point_in_rect(POINT { x: 60, y: 19 }, rect));
            assert!(!point_in_rect(POINT { x: 60, y: 121 }, rect));
        }
    }
}

/// A named screen anchor. `BottomRight` is also the fallback for any
/// unrecognized position string, matching the shipped default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NamedCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

/// Parsed form of `DesktopPetConfig::position`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionSpec {
    /// A named anchor such as `bottom-right`.
    Named(NamedCorner),
    /// Legacy absolute physical pixels (`"x,y"`).
    Absolute(i32, i32),
    /// Ratios in `[0,1]` relative to the active monitor's work area
    /// (`"rel:rx,ry"`). Display-resolution independent.
    Relative(f64, f64),
}

/// Parse the free-form `position` config value. Accepted forms:
/// `rel:<rx>,<ry>` (ratios), `<x>,<y>` (legacy absolute pixels), or a named
/// anchor (`top-left`, `top-right`, `bottom-left`, `bottom-right`, `center`).
pub fn parse_position(raw: &str) -> PositionSpec {
    let trimmed = raw.trim();

    if let Some(rest) = trimmed.strip_prefix("rel:") {
        if let Some((rx_str, ry_str)) = rest.split_once(',') {
            if let (Ok(rx), Ok(ry)) = (rx_str.trim().parse::<f64>(), ry_str.trim().parse::<f64>()) {
                if rx.is_finite() && ry.is_finite() {
                    return PositionSpec::Relative(rx.clamp(0.0, 1.0), ry.clamp(0.0, 1.0));
                }
            }
        }
    }

    if let Some((x_str, y_str)) = trimmed.split_once(',') {
        if let (Ok(x), Ok(y)) = (x_str.trim().parse::<i32>(), y_str.trim().parse::<i32>()) {
            return PositionSpec::Absolute(x, y);
        }
    }

    let corner = match trimmed {
        "top-left" => NamedCorner::TopLeft,
        "top-right" => NamedCorner::TopRight,
        "bottom-left" => NamedCorner::BottomLeft,
        "center" => NamedCorner::Center,
        _ => NamedCorner::BottomRight,
    };
    PositionSpec::Named(corner)
}

/// Serialize a relative position back to the persisted `rel:rx,ry` form.
pub fn format_relative(rx: f64, ry: f64) -> String {
    format!("rel:{:.4},{:.4}", rx, ry)
}

fn window_physical_size(window: &tauri::WebviewWindow) -> (i32, i32) {
    let width = window.outer_size().map(|s| s.width as i32).unwrap_or(320);
    let height = window.outer_size().map(|s| s.height as i32).unwrap_or(500);
    (width, height)
}

/// The monitor that should host a relative/named position: the one the window
/// is currently on, falling back to the primary monitor.
fn active_monitor(window: &tauri::WebviewWindow) -> Option<tauri::Monitor> {
    window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
}

fn anchor_in_work_area(
    corner: NamedCorner,
    monitor: &tauri::Monitor,
    width: i32,
    height: i32,
) -> tauri::PhysicalPosition<i32> {
    let work = monitor.work_area();
    let left = work.position.x;
    let top = work.position.y;
    let right = (left + work.size.width as i32 - width).max(left);
    let bottom = (top + work.size.height as i32 - height).max(top);
    let mid_x = (left + (work.size.width as i32 - width) / 2).clamp(left, right);
    let mid_y = (top + (work.size.height as i32 - height) / 2).clamp(top, bottom);

    match corner {
        NamedCorner::TopLeft => tauri::PhysicalPosition::new(left, top),
        NamedCorner::TopRight => tauri::PhysicalPosition::new(right, top),
        NamedCorner::BottomLeft => tauri::PhysicalPosition::new(left, bottom),
        NamedCorner::BottomRight => tauri::PhysicalPosition::new(right, bottom),
        NamedCorner::Center => tauri::PhysicalPosition::new(mid_x, mid_y),
    }
}

/// Resolve a position spec to concrete physical pixels on the active monitor.
pub fn resolve_position(
    window: &tauri::WebviewWindow,
    spec: PositionSpec,
) -> tauri::PhysicalPosition<i32> {
    let (width, height) = window_physical_size(window);

    match spec {
        PositionSpec::Absolute(x, y) => {
            clamp_to_visible_work_area(window, tauri::PhysicalPosition::new(x, y))
        }
        PositionSpec::Named(corner) => match active_monitor(window) {
            Some(monitor) => anchor_in_work_area(corner, &monitor, width, height),
            None => tauri::PhysicalPosition::new(0, 0),
        },
        PositionSpec::Relative(rx, ry) => match active_monitor(window) {
            Some(monitor) => {
                let work = *monitor.work_area();
                // (rx, ry) describe the window's BOTTOM-CENTER as a fraction of
                // the work area. The bottom edge is the stable anchor: bubbles
                // and the dashboard stack upward, and anchored resizes keep the
                // bottom-center fixed, so it doesn't move when the window grows.
                let bottom_center_x =
                    work.position.x + (rx * work.size.width as f64).round() as i32;
                let bottom_y = work.position.y + (ry * work.size.height as f64).round() as i32;
                let x = bottom_center_x - width / 2;
                let y = bottom_y - height;
                clamp_to_visible_work_area(window, tauri::PhysicalPosition::new(x, y))
            }
            None => tauri::PhysicalPosition::new(0, 0),
        },
    }
}

/// If `pos` is within `threshold` physical pixels of a work-area corner, return
/// that corner's exact position so the caller can snap the window to it.
#[allow(dead_code)]
pub fn snap_to_corner(
    window: &tauri::WebviewWindow,
    pos: tauri::PhysicalPosition<i32>,
    threshold: f32,
) -> Option<tauri::PhysicalPosition<i32>> {
    if !threshold.is_finite() || threshold <= 0.0 {
        return None;
    }

    let monitor = window
        .monitor_from_point(pos.x as f64, pos.y as f64)
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;

    let (width, height) = window_physical_size(window);
    let corners = [
        anchor_in_work_area(NamedCorner::TopLeft, &monitor, width, height),
        anchor_in_work_area(NamedCorner::TopRight, &monitor, width, height),
        anchor_in_work_area(NamedCorner::BottomLeft, &monitor, width, height),
        anchor_in_work_area(NamedCorner::BottomRight, &monitor, width, height),
    ];

    let mut best: Option<(f64, tauri::PhysicalPosition<i32>)> = None;
    for corner in corners {
        let dx = (corner.x - pos.x) as f64;
        let dy = (corner.y - pos.y) as f64;
        let distance = (dx * dx + dy * dy).sqrt();
        if best.is_none_or(|(best_distance, _)| distance < best_distance) {
            best = Some((distance, corner));
        }
    }

    best.filter(|(distance, _)| *distance <= threshold as f64)
        .map(|(_, corner)| corner)
}

/// Per-side escape latch for the edge snap. While a drag holds the pet pinned
/// flush against a work-area edge, every Moved event re-snaps it back to the
/// edge and the pet cannot be dragged out; once the drag pulls the pet away
/// from an edge it was sitting exactly on, that edge is marked escaped and
/// stops grabbing until the drag leaves its snap zone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EdgeEscape {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

/// Escape latch for the drag in progress. Lives at module level because the
/// Windows mouse hook must reset it on a fresh left-button press (a drag can
/// end without a final Moved event, which would otherwise leave a stale latch
/// that silently disables snapping for the next drag). The critical section is
/// a few bytes of copying, so contention with the hook thread is negligible.
static DRAG_ESCAPE: Mutex<EdgeEscape> = Mutex::new(EdgeEscape {
    left: false,
    right: false,
    top: false,
    bottom: false,
});

/// Snapshot of the current drag's escape latch.
pub(crate) fn drag_escape() -> EdgeEscape {
    *DRAG_ESCAPE.lock().unwrap()
}

/// Replace the current drag's escape latch.
pub(crate) fn set_drag_escape(escaped: EdgeEscape) {
    *DRAG_ESCAPE.lock().unwrap() = escaped;
}

/// Clear the escape latch. Called on a fresh left-button press (Windows hook),
/// when no drag is running, and after a drag ends.
pub(crate) fn reset_drag_escape() {
    *DRAG_ESCAPE.lock().unwrap() = EdgeEscape::default();
}

/// Outcome of one snap decision: the position to apply and the escape flags
/// to carry into the next event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapResult {
    pub position: tauri::PhysicalPosition<i32>,
    pub escaped: EdgeEscape,
}

/// Pure snap decision for a single drag event. `pos` is the candidate
/// top-left, `prev` the last applied top-left (None when unknown — the first
/// event of a drag), `work` the monitor work area `(left, top, right,
/// bottom)` and `size` the window's outer size, all in physical pixels.
/// Returns the position to apply (snapped toward an approaching edge,
/// unchanged otherwise) plus the updated escape flags.
fn snap_position(
    pos: (i32, i32),
    prev: Option<(i32, i32)>,
    work: (i32, i32, i32, i32),
    size: (i32, i32),
    threshold: i32,
    mut escaped: EdgeEscape,
) -> ((i32, i32), EdgeEscape) {
    let (work_left, work_top, work_right, work_bottom) = work;
    let edge_left_x = work_left;
    let edge_right_x = work_right - size.0;
    let edge_top_y = work_top;
    let edge_bottom_y = work_bottom - size.1;

    // Broke free: the pet was sitting exactly on an edge and the drag has now
    // pulled it inward — that edge lets go for the rest of this drag.
    if let Some((prev_x, prev_y)) = prev {
        if prev_x == edge_left_x && pos.0 > prev_x {
            escaped.left = true;
        }
        if prev_x == edge_right_x && pos.0 < prev_x {
            escaped.right = true;
        }
        if prev_y == edge_top_y && pos.1 > prev_y {
            escaped.top = true;
        }
        if prev_y == edge_bottom_y && pos.1 < prev_y {
            escaped.bottom = true;
        }
    }

    // Once the pet is far from an axis's edges, that axis's latch has served
    // its purpose and snapping can grab again on approach.
    if (pos.0 - edge_left_x).abs() > threshold && (pos.0 - edge_right_x).abs() > threshold {
        escaped.left = false;
        escaped.right = false;
    }
    if (pos.1 - edge_top_y).abs() > threshold && (pos.1 - edge_bottom_y).abs() > threshold {
        escaped.top = false;
        escaped.bottom = false;
    }

    let mut x = pos.0;
    let mut y = pos.1;
    if !escaped.left && (x - edge_left_x).abs() <= threshold {
        x = edge_left_x;
    } else if !escaped.right && (x - edge_right_x).abs() <= threshold {
        x = edge_right_x;
    }
    if !escaped.top && (y - edge_top_y).abs() <= threshold {
        y = edge_top_y;
    } else if !escaped.bottom && (y - edge_bottom_y).abs() <= threshold {
        y = edge_bottom_y;
    }

    ((x, y), escaped)
}

/// Snap the window position to the nearest work-area **edge** when within
/// `threshold` physical pixels. Unlike `snap_to_corner` (which requires
/// proximity to a corner on both axes), this snaps each axis independently so
/// the user can drag to any screen edge and feel an immediate magnetic pull.
/// Edges the current drag has already escaped from are skipped (see
/// [`EdgeEscape`]). Returns `None` when snapping is disabled or the monitor
/// cannot be determined — the caller then keeps the dragged position and the
/// escape flags unchanged.
pub fn snap_to_edges(
    window: &tauri::WebviewWindow,
    pos: tauri::PhysicalPosition<i32>,
    prev: Option<(i32, i32)>,
    escaped: EdgeEscape,
    threshold: f32,
) -> Option<SnapResult> {
    if !threshold.is_finite() || threshold <= 0.0 {
        return None;
    }

    let monitor = window
        .monitor_from_point(pos.x as f64, pos.y as f64)
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;

    let (width, height) = window_physical_size(window);
    let work = *monitor.work_area();
    let ((x, y), escaped) = snap_position(
        (pos.x, pos.y),
        prev,
        (
            work.position.x,
            work.position.y,
            work.position.x + work.size.width as i32,
            work.position.y + work.size.height as i32,
        ),
        (width, height),
        threshold as i32,
        escaped,
    );
    Some(SnapResult {
        position: tauri::PhysicalPosition::new(x, y),
        escaped,
    })
}

/// Express a physical position as `(rx, ry)` ratios within the work area of the
/// monitor that contains it. Used to persist a resolution-independent position.
pub fn position_to_relative(
    window: &tauri::WebviewWindow,
    pos: tauri::PhysicalPosition<i32>,
) -> Option<(f64, f64)> {
    let monitor = window
        .monitor_from_point(pos.x as f64, pos.y as f64)
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;

    let (width, height) = window_physical_size(window);
    let work = *monitor.work_area();
    let span_x = (work.size.width as i32).max(1);
    let span_y = (work.size.height as i32).max(1);
    // Persist the BOTTOM-CENTER of the window as a fraction of the work area:
    // the pet grows upward, so the bottom edge is the point that stays put
    // across resizes and restarts.
    let bottom_center_x = pos.x + width / 2;
    let bottom_y = pos.y + height;
    let rx = ((bottom_center_x - work.position.x) as f64 / span_x as f64).clamp(0.0, 1.0);
    let ry = ((bottom_y - work.position.y) as f64 / span_y as f64).clamp(0.0, 1.0);
    Some((rx, ry))
}

/// Whether the window currently intersects at least one visible monitor's work
/// area. The position watcher uses this to decide whether a position needs
/// correcting: a pet that is still on screen is left untouched (so unrelated
/// monitor changes never move it); only a pet stranded off-screen — e.g. after
/// its monitor was unplugged — gets pulled back.
pub fn is_within_visible_work_area(window: &tauri::WebviewWindow) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return true;
    };
    if monitors.is_empty() {
        // Cannot tell; assume fine rather than yank the window around.
        return true;
    }
    let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else {
        return true;
    };

    let left = pos.x;
    let top = pos.y;
    let right = pos.x + size.width as i32;
    let bottom = pos.y + size.height as i32;

    monitors.iter().any(|monitor| {
        let work = monitor.work_area();
        let work_left = work.position.x;
        let work_top = work.position.y;
        let work_right = work_left + work.size.width as i32;
        let work_bottom = work_top + work.size.height as i32;
        left < work_right && right > work_left && top < work_bottom && bottom > work_top
    })
}

/// Clamp a requested physical top-left position into the work area of the
/// monitor nearest to that point. A stale saved coordinate — a monitor that
/// was unplugged, a resolution or DPI change, or a window dragged below the
/// taskbar — would otherwise restore the pet outside every visible monitor,
/// where the user can neither see nor grab it.
fn clamp_to_visible_work_area(
    window: &tauri::WebviewWindow,
    requested: tauri::PhysicalPosition<i32>,
) -> tauri::PhysicalPosition<i32> {
    let monitor = window
        .monitor_from_point(requested.x as f64, requested.y as f64)
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return requested;
    };

    let work = *monitor.work_area();
    let width = window.outer_size().map(|s| s.width as i32).unwrap_or(320);
    let height = window.outer_size().map(|s| s.height as i32).unwrap_or(500);

    let left = work.position.x;
    let top = work.position.y;
    let right = left + work.size.width as i32;
    let bottom = top + work.size.height as i32;

    let x = requested.x.clamp(left, (right - width).max(left));
    let y = requested.y.clamp(top, (bottom - height).max(top));

    tauri::PhysicalPosition::new(x, y)
}

/// Keep a dragged window on screen. The candidate position is allowed when the
/// window rect stays fully inside one monitor's work area (normal movement) or
/// intersects two monitors' full frames (crossing between monitors); otherwise
/// it is clamped into the nearest work area so the pet can never be dragged
/// off-screen. Returns `None` when the position needs no correction.
pub fn clamp_drag_position(
    window: &tauri::WebviewWindow,
    pos: tauri::PhysicalPosition<i32>,
) -> Option<tauri::PhysicalPosition<i32>> {
    let monitors = window.available_monitors().ok()?;
    if monitors.is_empty() {
        return None;
    }
    let (width, height) = window_physical_size(window);
    let (left, top, right, bottom) = (pos.x, pos.y, pos.x + width, pos.y + height);

    let mut intersections = 0;
    let mut fully_inside = false;
    for monitor in &monitors {
        // Full-frame intersection: detects monitor crossing (work areas can
        // have gaps, e.g. a dock, so they must not gate crossing).
        let frame_pos = monitor.position();
        let frame_size = monitor.size();
        let (fl, ft) = (frame_pos.x, frame_pos.y);
        let (fr, fb) = (fl + frame_size.width as i32, ft + frame_size.height as i32);
        if left < fr && right > fl && top < fb && bottom > ft {
            intersections += 1;
        }
        let work = monitor.work_area();
        let (wl, wt) = (work.position.x, work.position.y);
        let (wr, wb) = (wl + work.size.width as i32, wt + work.size.height as i32);
        if left >= wl && right <= wr && top >= wt && bottom <= wb {
            fully_inside = true;
        }
    }

    if fully_inside || intersections >= 2 {
        None
    } else {
        Some(clamp_to_visible_work_area(window, pos))
    }
}

// --- Click-through via system hit-test transparency ------------------------
//
// `ignoresMouseEvents` is the only reliable pass-through: the window server
// skips the panel during hit-testing entirely, so clicks reach the window
// behind — no event re-injection, no Accessibility permission, no timers.
// (The previous CGEventPost re-injection design silently dropped clicks:
// synthetic event injection is TCC-gated, and re-posted events could loop
// back into our own window and be consumed.)
//
// The price of ignoresMouseEvents is that ALL buttons pass through, so while
// pass-through is on a system-wide right-click monitor watches right-downs
// over the panel frame, makes the panel live again and asks the front-end to
// open the context menu at the cursor. The menu suspends pass-through while
// open (set_enabled(false) from the front-end), keeping its items clickable.
#[cfg(target_os = "macos")]
mod click_through {
    use cocoa::base::id;
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use serde::Serialize;
    use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
    use std::sync::OnceLock;
    use tauri::{Emitter, Manager};

    static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
    static WINDOW: OnceLock<tauri::WebviewWindow> = OnceLock::new();
    /// The pet panel pointer; used by the monitor and the ignoresMouseEvents
    /// toggle.
    static PANEL: AtomicPtr<Object> = AtomicPtr::new(std::ptr::null_mut());
    /// Effective pass-through state (suspended while the context menu is open).
    static ENABLED: AtomicBool = AtomicBool::new(false);
    /// Keeps the global right-click monitor alive for the process lifetime.
    #[allow(dead_code)]
    static RIGHT_MONITOR: AtomicPtr<Object> = AtomicPtr::new(std::ptr::null_mut());

    /// Cursor position in window-relative CSS pixels for the front-end menu.
    #[derive(Clone, Serialize)]
    struct ContextMenuPoint {
        x: f64,
        y: f64,
    }

    /// Toggle pass-through at runtime; applies `ignoresMouseEvents` on the
    /// main thread. Called from config apply and from the front-end when the
    /// context menu opens (false) / closes (configured value).
    pub fn set_enabled(enabled: bool) {
        if ENABLED.swap(enabled, Ordering::SeqCst) == enabled {
            return;
        }
        tracing::info!(enabled, "click_through: pass-through state changed");
        let Some(window) = WINDOW.get() else {
            tracing::warn!("click_through: not installed yet; state stored only");
            return;
        };
        let window = window.clone();
        let _ = window.run_on_main_thread(move || {
            let panel = PANEL.load(Ordering::SeqCst);
            if panel.is_null() {
                return;
            }
            unsafe {
                let _: () = msg_send![
                    panel,
                    setIgnoresMouseEvents: if enabled { cocoa::base::YES } else { cocoa::base::NO }
                ];
            }
        });
    }

    /// Global right-down monitor callback (main thread). With pass-through on
    /// the panel is skipped by hit-testing, so the right-click is headed to
    /// the app behind; we observe it here, make the panel live again and ask
    /// the front-end to open the context menu at the cursor.
    unsafe fn handle_right_down() {
        if !ENABLED.load(Ordering::SeqCst) {
            // Window is live; the webview's own contextmenu handler runs.
            return;
        }
        let panel = PANEL.load(Ordering::SeqCst);
        if panel.is_null() {
            return;
        }
        let mouse: cocoa::foundation::NSPoint = msg_send![class!(NSEvent), mouseLocation];
        let frame: cocoa::foundation::NSRect = msg_send![panel, frame];
        let inside = mouse.x >= frame.origin.x
            && mouse.x <= frame.origin.x + frame.size.width
            && mouse.y >= frame.origin.y
            && mouse.y <= frame.origin.y + frame.size.height;
        if !inside {
            tracing::debug!("click_through: right-click outside pet; ignored");
            return;
        }
        tracing::info!("click_through: right-click over pet; opening context menu");
        let _: () = msg_send![panel, setIgnoresMouseEvents: cocoa::base::NO];
        // Cursor relative to the window's top-left, in points (== CSS px).
        let point = ContextMenuPoint {
            x: mouse.x - frame.origin.x,
            y: frame.origin.y + frame.size.height - mouse.y,
        };
        if let Some(app) = APP.get() {
            let _ = app.emit("pet_context_menu", point);
        }
    }

    /// Install the panel pointer and the global right-click monitor.
    /// Idempotent.
    pub fn install(window: &tauri::WebviewWindow) -> Result<(), String> {
        let _ = APP.set(window.app_handle().clone());
        let _ = WINDOW.set(window.clone());
        let window_for_main = window.clone();
        window
            .run_on_main_thread(move || {
                use cocoa::appkit::NSWindow;

                unsafe {
                    let Ok(ns_window_ptr) = window_for_main.ns_window() else {
                        tracing::warn!("click_through: no NSWindow available");
                        return;
                    };
                    let ns_window = ns_window_ptr as id;
                    PANEL.store(ns_window, Ordering::SeqCst);

                    // Keep mouse-moved events flowing to the webview (hover).
                    ns_window.setAcceptsMouseMovedEvents_(cocoa::base::YES);

                    static ONCE: std::sync::Once = std::sync::Once::new();
                    ONCE.call_once(|| {
                        // NSEventMaskRightMouseDown = 1 << NSEventTypeRightMouseDown(3).
                        let mask: u64 = 1 << 3;
                        let block = block::ConcreteBlock::new(|_event: id| {
                            handle_right_down();
                        })
                        .copy();
                        let monitor: id = msg_send![
                            class!(NSEvent),
                            addGlobalMonitorForEventsMatchingMask: mask
                            handler: &*block
                        ];
                        RIGHT_MONITOR.store(monitor, Ordering::SeqCst);
                        tracing::info!("click_through: global right-click monitor installed");
                    });
                }
            })
            .map_err(|error| error.to_string())
    }
}
#[cfg(target_os = "macos")]
fn apply_macos_desktop_behavior(
    window: &tauri::WebviewWindow,
    show_on_all_desktops: bool,
) -> Result<(), String> {
    let window_for_main_thread = window.clone();
    window
        .run_on_main_thread(move || {
            use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
            use cocoa::base::id;

            let Ok(ns_window_ptr) = window_for_main_thread.ns_window() else {
                tracing::warn!("failed to access the desktop pet NSPanel");
                return;
            };
            let ns_window = ns_window_ptr as id;

            unsafe {
                let managed_behaviors =
                    NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary;
                let mut behavior = ns_window.collectionBehavior();

                if show_on_all_desktops {
                    behavior.insert(managed_behaviors);
                } else {
                    behavior.remove(managed_behaviors);
                }

                ns_window.setCollectionBehavior_(behavior);
                tracing::info!(
                    show_on_all_desktops,
                    collection_behavior = behavior.bits(),
                    "applied desktop pet macOS collection behavior"
                );
            }
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod snap_tests {
    use super::*;

    // 1920x1080 work area, 200x300 pet window, 60px snap threshold.
    fn work() -> (i32, i32, i32, i32) {
        (0, 0, 1920, 1080)
    }

    fn size() -> (i32, i32) {
        (200, 300)
    }

    const THRESHOLD: i32 = 60;

    #[test]
    fn snaps_when_approaching_an_edge() {
        // Right edge sits at 1920 - 200 = 1720; 1750 is within the threshold.
        let ((x, y), escaped) = snap_position(
            (1750, 400),
            Some((1810, 400)),
            work(),
            size(),
            THRESHOLD,
            EdgeEscape::default(),
        );
        assert_eq!((x, y), (1720, 400));
        assert_eq!(escaped, EdgeEscape::default());
    }

    #[test]
    fn snaps_nothing_when_far_from_edges() {
        let ((x, y), escaped) = snap_position(
            (500, 400),
            Some((500, 400)),
            work(),
            size(),
            THRESHOLD,
            EdgeEscape::default(),
        );
        assert_eq!((x, y), (500, 400));
        assert_eq!(escaped, EdgeEscape::default());
    }

    #[test]
    fn escapes_right_edge_when_pulled_away() {
        // Pet pinned flush against the right edge (x = 1720), user pulls left.
        let ((x, y), escaped) = snap_position(
            (1700, 400),
            Some((1720, 400)),
            work(),
            size(),
            THRESHOLD,
            EdgeEscape::default(),
        );
        assert_eq!(
            (x, y),
            (1700, 400),
            "pulled position must not be re-snapped"
        );
        assert!(escaped.right);
        assert!(!escaped.left);
    }

    #[test]
    fn escaped_edge_does_not_regrab_within_zone() {
        let escaped = EdgeEscape {
            right: true,
            ..EdgeEscape::default()
        };
        // Still inside the right-edge snap zone; must stay free.
        let ((x, _), _) = snap_position(
            (1750, 400),
            Some((1720, 400)),
            work(),
            size(),
            THRESHOLD,
            escaped,
        );
        assert_eq!(x, 1750);
    }

    #[test]
    fn escape_latch_resets_in_free_space() {
        let escaped = EdgeEscape {
            right: true,
            ..EdgeEscape::default()
        };
        let (_, escaped) = snap_position(
            (1500, 400), // > 60px away from both x edges (1720 and 0)
            Some((1600, 400)),
            work(),
            size(),
            THRESHOLD,
            escaped,
        );
        assert!(!escaped.right);
    }

    #[test]
    fn approaches_corner_snaps_both_axes() {
        let ((x, y), escaped) = snap_position(
            (1770, 830), // 50px from right edge (1720) and bottom edge (780)
            Some((1740, 800)),
            work(),
            size(),
            THRESHOLD,
            EdgeEscape::default(),
        );
        assert_eq!((x, y), (1720, 780));
        assert_eq!(escaped, EdgeEscape::default());
    }

    #[test]
    fn escapes_bottom_edge_when_pulled_up() {
        // Pull up 40px: still inside the bottom-edge snap zone, so the latch
        // must persist and the position must not be re-snapped down.
        let ((_, y), escaped) = snap_position(
            (400, 740),
            Some((400, 780)),
            work(),
            size(),
            THRESHOLD,
            EdgeEscape::default(),
        );
        assert_eq!(y, 740);
        assert!(escaped.bottom);
    }

    #[test]
    fn escape_latch_clears_once_past_the_snap_zone() {
        // Pull up 80px: beyond the 60px threshold, so the latch has served its
        // purpose (and nothing would re-snap at this distance anyway).
        let (_, escaped) = snap_position(
            (400, 700),
            Some((400, 780)),
            work(),
            size(),
            THRESHOLD,
            EdgeEscape::default(),
        );
        assert!(!escaped.bottom);
    }

    #[test]
    fn zero_threshold_never_snaps() {
        let ((x, y), escaped) = snap_position(
            (1750, 400),
            Some((1720, 400)),
            work(),
            size(),
            0,
            EdgeEscape::default(),
        );
        assert_eq!((x, y), (1750, 400));
        assert_eq!(escaped, EdgeEscape::default());
    }

    #[test]
    fn drag_escape_global_state_roundtrips_and_resets() {
        // This is the only test touching the shared latch; cargo runs tests
        // in one process, so it must restore the default before returning.
        set_drag_escape(EdgeEscape {
            right: true,
            ..EdgeEscape::default()
        });
        assert_eq!(
            drag_escape(),
            EdgeEscape {
                right: true,
                ..EdgeEscape::default()
            }
        );
        reset_drag_escape();
        assert_eq!(drag_escape(), EdgeEscape::default());
    }
}
