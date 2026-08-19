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
) -> Result<(), String> {
    window
        .set_always_on_top(config.always_on_top)
        .map_err(|error| error.to_string())?;
    window
        .set_visible_on_all_workspaces(config.show_on_all_desktops)
        .map_err(|error| error.to_string())?;
    window
        .set_ignore_cursor_events(config.click_through)
        .map_err(|error| error.to_string())?;

    if let Some((x_str, y_str)) = config.position.split_once(',') {
        if let (Ok(x), Ok(y)) = (x_str.parse::<i32>(), y_str.parse::<i32>()) {
            let requested = tauri::PhysicalPosition::new(x, y);
            let visible = clamp_to_visible_work_area(window, requested);
            let _ = window.set_position(tauri::Position::Physical(visible));
        }
    }

    #[cfg(target_os = "macos")]
    apply_macos_desktop_behavior(window, config.show_on_all_desktops)?;

    Ok(())
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
