use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, Manager,
};

// Windows renders a blank tray square when no icon is provided, so embed the 32x32 PNG at compile time.
const TRAY_ICON: &[u8] = include_bytes!("../icons/32x32.png");

pub fn create_tray(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let lang = app
        .state::<std::sync::Arc<crate::commands::AppConfigState>>()
        .get_config()
        .general
        .language;
    let (settings_label, onboard_label, check_label) = if lang.starts_with("zh") {
        ("设置", "引导面板", "检查更新")
    } else {
        ("Settings", "Onboarding", "Check for Updates")
    };

    let settings_i = MenuItem::with_id(app, "settings", settings_label, true, None::<&str>)?;
    let onboard_i = MenuItem::with_id(app, "onboard", onboard_label, true, None::<&str>)?;
    let check_i = MenuItem::with_id(app, "check_update", check_label, true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_i, &onboard_i, &check_i, &quit_i])?;

    let _tray = TrayIconBuilder::new()
        .icon(Image::from_bytes(TRAY_ICON)?)
        .menu(&menu)
        .on_menu_event(|app, event| {
            let handle = app.clone();
            match event.id.as_ref() {
                "quit" => app.exit(0),
                "settings" => {
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = crate::commands::open_settings_window(handle, false).await {
                            tracing::warn!("failed to open settings window from tray: {e}");
                        }
                    });
                }
                "onboard" => {
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = crate::commands::open_onboard_window(handle).await {
                            tracing::warn!("failed to open onboarding window from tray: {e}");
                        }
                    });
                }
                // Delegate the check to the settings window: it owns the update
                // check UI (modal for a new version, toast for up-to-date or
                // failure), so the tray just asks it to run.
                "check_update" => {
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = crate::commands::open_settings_window(handle, true).await {
                            tracing::warn!("failed to open settings window for update check: {e}");
                        }
                    });
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}
