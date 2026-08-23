use familiar_core::config::DesktopPetConfig;

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
    // Click-through: on macOS only LEFT clicks pass through (right-click menu
    // and hover stay live) via the hitTest override; other platforms fall back
    // to ignoring all cursor events.
    #[cfg(target_os = "macos")]
    {
        window
            .set_ignore_cursor_events(false)
            .map_err(|error| error.to_string())?;
        click_through::set_enabled(config.click_through);
        click_through::install(window)?;
        hover::install(window)?;
    }
    #[cfg(not(target_os = "macos"))]
    window
        .set_ignore_cursor_events(config.click_through)
        .map_err(|error| error.to_string())?;

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
                let bottom_y =
                    work.position.y + (ry * work.size.height as f64).round() as i32;
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
    let height = window
        .outer_size()
        .map(|s| s.height as i32)
        .unwrap_or(500);

    let left = work.position.x;
    let top = work.position.y;
    let right = left + work.size.width as i32;
    let bottom = top + work.size.height as i32;

    let x = requested.x.clamp(left, (right - width).max(left));
    let y = requested.y.clamp(top, (bottom - height).max(top));

    tauri::PhysicalPosition::new(x, y)
}

// --- Left-click-through with a live right-click menu -----------------------
//
// Blanket `set_ignore_cursor_events(true)` kills the right-click menu and any
// hover detection, because the window stops receiving events entirely.
// Instead we override the content view's `hitTest:` so each incoming event is
// judged individually: left mouse events return nil (fall through to the
// window behind) while right-click and mouse-move events hit-test normally,
// keeping the context menu and event-driven hover transparency alive.
#[cfg(target_os = "macos")]
mod click_through {
    use cocoa::foundation::NSPoint;
    use objc::runtime::{Class, Method, Object, Sel};
    use objc::{msg_send, sel, sel_impl};
    use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

    /// The pet panel pointer; only this window passes left clicks through.
    static PANEL: AtomicPtr<Object> = AtomicPtr::new(std::ptr::null_mut());
    /// Mirrors `DesktopPetConfig::click_through`. When false, hit-testing is
    /// untouched so dragging and the menu both behave normally.
    static ENABLED: AtomicBool = AtomicBool::new(false);
    /// Original `hitTest:` IMP, saved once so the override can delegate.
    static ORIGINAL_IMP: AtomicUsize = AtomicUsize::new(0);

    type HitTestFn = unsafe extern "C" fn(&Object, Sel, NSPoint) -> *mut Object;

    pub fn set_enabled(enabled: bool) {
        ENABLED.store(enabled, Ordering::SeqCst);
    }

    /// Replacement `hitTest:`. Left mouse events on the pet panel return nil so
    /// they reach the window behind; everything else delegates to the original
    /// implementation.
    unsafe extern "C" fn hit_test(this: &Object, cmd: Sel, point: NSPoint) -> *mut Object {
        let original: HitTestFn =
            std::mem::transmute::<usize, HitTestFn>(ORIGINAL_IMP.load(Ordering::SeqCst));

        if !ENABLED.load(Ordering::SeqCst) {
            return original(this, cmd, point);
        }
        let panel = PANEL.load(Ordering::SeqCst);
        if panel.is_null() {
            return original(this, cmd, point);
        }
        let window: cocoa::base::id = msg_send![this, window];
        if !std::ptr::eq(window, panel) {
            return original(this, cmd, point);
        }

        let event: cocoa::base::id = msg_send![cocoa::appkit::NSApp(), currentEvent];
        if !event.is_null() {
            use objc::Message;
            // `type` is a Rust keyword, so build the selector from a string.
            let type_sel = sel_impl!("type\0");
            let event_type: u64 = (&*event).send_message(type_sel, ()).unwrap_or(0);
            // 1 = LeftMouseDown, 2 = LeftMouseUp, 6 = LeftMouseDragged.
            if matches!(event_type, 1 | 2 | 6) {
                return std::ptr::null_mut();
            }
        }
        original(this, cmd, point)
    }

    /// Install the `hitTest:` override on the pet window's content view. Safe to
    /// call repeatedly; the swizzle itself happens once.
    pub fn install(window: &tauri::WebviewWindow) -> Result<(), String> {
        let window_for_main = window.clone();
        window
            .run_on_main_thread(move || {
                use cocoa::appkit::NSWindow;
                use cocoa::base::id;

                unsafe {
                    let Ok(ns_window_ptr) = window_for_main.ns_window() else {
                        tracing::warn!("click_through: no NSWindow available");
                        return;
                    };
                    let ns_window = ns_window_ptr as id;
                    PANEL.store(ns_window, Ordering::SeqCst);

                    // Keep mouse-moved events flowing to the webview so hover
                    // transparency stays event-driven while left clicks pass
                    // through.
                    ns_window.setAcceptsMouseMovedEvents_(cocoa::base::YES);

                    let content_view: id = ns_window.contentView();
                    if content_view.is_null() {
                        tracing::warn!("click_through: no content view");
                        return;
                    }
                    let cls: *mut Class = msg_send![content_view, class];
                    let method: *const Method = objc::runtime::class_getInstanceMethod(
                        cls as *const Class,
                        sel!(hitTest:),
                    );
                    if method.is_null() {
                        tracing::warn!("click_through: hitTest: selector missing");
                        return;
                    }

                    static ONCE: std::sync::Once = std::sync::Once::new();
                    ONCE.call_once(|| {
                        let original = objc::runtime::method_getImplementation(method);
                        ORIGINAL_IMP.store(original as usize, Ordering::SeqCst);
                        let replacement: objc::runtime::Imp =
                            std::mem::transmute::<HitTestFn, objc::runtime::Imp>(
                                hit_test as HitTestFn,
                            );
                        objc::runtime::method_setImplementation(
                            method as *mut Method,
                            replacement,
                        );
                        tracing::info!("click_through: hitTest override installed");
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
