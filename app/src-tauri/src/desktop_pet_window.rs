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

    #[cfg(target_os = "macos")]
    apply_macos_desktop_behavior(window, config.show_on_all_desktops)?;

    Ok(())
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
