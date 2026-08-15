use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, Emitter, Manager,
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
                        if let Err(e) = crate::commands::open_settings_window(handle).await {
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
                "check_update" => {
                    tauri::async_runtime::spawn(async move {
                        let config_state = handle
                            .state::<std::sync::Arc<crate::commands::AppConfigState>>()
                            .inner()
                            .clone();
                        let pending = handle
                            .state::<std::sync::Arc<crate::updates::PendingUpdateState>>()
                            .inner()
                            .clone();
                        match crate::updates::run_check(&handle, &config_state, &pending, true)
                            .await
                        {
                            Ok(result) if result.has_update => {
                                let _ = crate::commands::open_settings_window(handle.clone()).await;
                                let _ = handle.emit("update_available", &result);
                            }
                            Ok(_) => {}
                            Err(e) => tracing::warn!("tray update check failed: {e}"),
                        }
                    });
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}
