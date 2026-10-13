//! Only restore visibility that this feature itself suppressed.
use std::sync::Arc;
use tauri::Manager;

pub fn start(app: tauri::AppHandle, config: Arc<crate::commands::AppConfigState>) {
    tauri::async_runtime::spawn(async move {
        let mut hidden = false;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let Some(window) = app.get_webview_window("main") else {
                break;
            };
            let enabled = config.get_config().renderer.desktop_pet.hide_on_fullscreen;
            if !enabled && !hidden {
                continue;
            }
            // AppKit queries and visibility changes must run on the main thread.
            let (tx, rx) = tokio::sync::oneshot::channel();
            let target = window.clone();
            if window
                .run_on_main_thread(move || {
                    let suppress = enabled && is_fullscreen(&target);
                    let mut next = hidden;
                    if suppress {
                        if target.is_visible().unwrap_or(false)
                            && hide_for_fullscreen(&target).is_ok()
                        {
                            next = true;
                        }
                    } else if hidden && restore_without_focus(&target).is_ok() {
                        next = false;
                    }
                    let _ = tx.send(next);
                })
                .is_err()
            {
                break;
            }
            if let Ok(next) = rx.await {
                hidden = next;
            }
        }
    });
}

fn hide_for_fullscreen(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Pair native hide/show: Tauri caches visibility when hiding, so
        // mixing Tauri hide with native show makes subsequent hides no-ops.
        set_windows_visibility(window.hwnd().map_err(|error| error.to_string())?, false)
    }
    #[cfg(not(target_os = "windows"))]
    window.hide().map_err(|error| error.to_string())
}

/// Called on the main thread. Automatic restoration must preserve the user's
/// input target; Tauri's macOS `show()` calls `makeKeyAndOrderFront`.
fn restore_without_focus(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use cocoa::base::id;
        use objc::{msg_send, sel, sel_impl};

        let native = window.ns_window().map_err(|error| error.to_string())?;
        unsafe {
            let _: () = msg_send![native as id, orderFrontRegardless];
        }
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        set_windows_visibility(window.hwnd().map_err(|error| error.to_string())?, true)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    window.show().map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn set_windows_visibility(
    window: windows::Win32::Foundation::HWND,
    visible: bool,
) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{
        IsWindowVisible, ShowWindow, SW_HIDE, SW_SHOWNA,
    };
    unsafe {
        // ShowWindow returns the previous visibility, not a success flag.
        // SW_SHOWNA preserves size/position and never activates the pet.
        let _ = ShowWindow(window, if visible { SW_SHOWNA } else { SW_HIDE });
        if IsWindowVisible(window).as_bool() != visible {
            return Err("failed to update desktop pet visibility".to_string());
        }
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn fills_screen(window: (f64, f64, f64, f64), screen: (f64, f64, f64, f64)) -> bool {
    // Allow rounding at fractional DPI, but exclude ordinary maximized windows.
    (window.0 - screen.0).abs() <= 2.0
        && (window.1 - screen.1).abs() <= 2.0
        && (window.2 - screen.2).abs() <= 2.0
        && (window.3 - screen.3).abs() <= 2.0
}

#[cfg(target_os = "macos")]
fn is_fullscreen(window: &tauri::WebviewWindow) -> bool {
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSRect;
    use objc::{class, msg_send, sel, sel_impl};
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowListCopyWindowInfo(options: u32, relative: u32) -> id;
        fn CGDisplayBounds(display: u32) -> NSRect;
    }
    unsafe {
        let Ok(ptr) = window.ns_window() else {
            return false;
        };
        let screen: id = msg_send![ptr as id, screen];
        if screen == nil {
            return false;
        }
        let description: id = msg_send![screen, deviceDescription];
        let key = cocoa::foundation::NSString::alloc(nil);
        use cocoa::foundation::NSString;
        let key = key.init_str("NSScreenNumber");
        let number: id = msg_send![description, objectForKey: key];
        let _: () = msg_send![key, release];
        if number == nil {
            return false;
        }
        let display: u32 = msg_send![number, unsignedIntValue];
        let bounds = CGDisplayBounds(display);
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let front: id = msg_send![workspace, frontmostApplication];
        if front == nil {
            return false;
        }
        let pid: i32 = msg_send![front, processIdentifier];
        if pid == std::process::id() as i32 {
            return false;
        }
        // Onscreen only + exclude desktop elements. Bounds/PID/layer are available
        // without reading titles or capturing any screen content.
        let list = CGWindowListCopyWindowInfo(1 | 16, 0);
        if list == nil {
            return false;
        }
        let count: usize = msg_send![list, count];
        let mut result = false;
        for i in 0..count {
            let entry: id = msg_send![list, objectAtIndex: i];
            let value = |name: &str| -> id {
                let key = NSString::alloc(nil).init_str(name);
                let val: id = msg_send![entry, objectForKey: key];
                let _: () = msg_send![key, release];
                val
            };
            let owner = value("kCGWindowOwnerPID");
            let layer = value("kCGWindowLayer");
            if owner == nil || layer == nil {
                continue;
            }
            let owner: i32 = msg_send![owner, intValue];
            let layer: i32 = msg_send![layer, intValue];
            if owner != pid || layer != 0 {
                continue;
            }
            let rect = value("kCGWindowBounds");
            if rect == nil {
                continue;
            }
            let component = |name: &str| -> f64 {
                let key = NSString::alloc(nil).init_str(name);
                let val: id = msg_send![rect, objectForKey: key];
                let _: () = msg_send![key, release];
                if val == nil {
                    return f64::NAN;
                }
                msg_send![val, doubleValue]
            };
            let rect = (
                component("X"),
                component("Y"),
                component("Width"),
                component("Height"),
            );
            let screen = (
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width,
                bounds.size.height,
            );
            if rect.0 < screen.0 + screen.2
                && rect.0 + rect.2 > screen.0
                && rect.1 < screen.1 + screen.3
                && rect.1 + rect.3 > screen.1
            {
                result = fills_screen(rect, screen);
                break;
            }
        }
        let _: () = msg_send![list, release];
        result
    }
}

#[cfg(target_os = "windows")]
fn is_fullscreen(window: &tauri::WebviewWindow) -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetShellWindow, GetWindowRect, GetWindowThreadProcessId,
    };
    unsafe {
        let front = GetForegroundWindow();
        if front.0.is_null() || front == GetShellWindow() {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(front, Some(&mut pid));
        if pid == std::process::id() {
            return false;
        }
        let mut rect = RECT::default();
        if GetWindowRect(front, &mut rect).is_err() {
            return false;
        }
        let Ok(Some(monitor)) = window.current_monitor() else {
            return false;
        };
        let pos = monitor.position();
        let size = monitor.size();
        fills_screen(
            (
                rect.left as f64,
                rect.top as f64,
                (rect.right - rect.left) as f64,
                (rect.bottom - rect.top) as f64,
            ),
            (
                pos.x as f64,
                pos.y as f64,
                size.width as f64,
                size.height as f64,
            ),
        )
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn is_fullscreen(_window: &tauri::WebviewWindow) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    #[test]
    fn repeated_windows_restoration_preserves_foreground_window() {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GetForegroundWindow, IsWindowVisible, WINDOW_EX_STYLE,
            WS_POPUP,
        };
        unsafe {
            let native = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Familiar visibility test"),
                WS_POPUP,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .expect("create hidden test window");
            struct TestWindow(windows::Win32::Foundation::HWND);
            impl Drop for TestWindow {
                fn drop(&mut self) {
                    unsafe {
                        let _ = DestroyWindow(self.0);
                    }
                }
            }
            let _guard = TestWindow(native);
            let foreground = GetForegroundWindow();
            for _ in 0..3 {
                set_windows_visibility(native, true).expect("restore without activation");
                assert!(IsWindowVisible(native).as_bool());
                assert_eq!(GetForegroundWindow(), foreground);
                set_windows_visibility(native, false).expect("hide again");
                assert!(!IsWindowVisible(native).as_bool());
                assert_eq!(GetForegroundWindow(), foreground);
            }
        }
    }

    #[test]
    fn distinguishes_fullscreen_from_maximized_and_other_monitors() {
        let screen = (0., 0., 1920., 1080.);
        assert!(fills_screen(screen, screen));
        assert!(fills_screen((1., 0., 1919., 1080.), screen));
        assert!(!fills_screen((0., 25., 1920., 1015.), screen));
        assert!(!fills_screen((1920., 0., 1920., 1080.), screen));
        assert!(!fills_screen((f64::NAN, 0., 1920., 1080.), screen));
    }
}
