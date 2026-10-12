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
                        if target.is_visible().unwrap_or(false) && target.hide().is_ok() {
                            next = true;
                        }
                    } else if hidden && target.show().is_ok() {
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
