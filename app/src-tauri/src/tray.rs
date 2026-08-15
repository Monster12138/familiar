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
    let check_label = if lang.starts_with("zh") {
        "检查更新"
    } else {
        "Check for Updates"
    };

    let check_i = MenuItem::with_id(app, "check_update", check_label, true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&check_i, &quit_i])?;

    let _tray = TrayIconBuilder::new()
        .icon(Image::from_bytes(TRAY_ICON)?)
        .menu(&menu)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
            } else if event.id.as_ref() == "check_update" {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let config_state = handle
                        .state::<std::sync::Arc<crate::commands::AppConfigState>>()
                        .inner()
                        .clone();
                    let pending = handle
                        .state::<std::sync::Arc<crate::updates::PendingUpdateState>>()
                        .inner()
                        .clone();
                    match crate::updates::run_check(&handle, &config_state, &pending, true).await {
                        Ok(result) if result.has_update => {
                            let _ = crate::commands::open_settings_window(handle.clone()).await;
                            let _ = handle.emit("update_available", &result);
                        }
                        Ok(_) => {}
                        Err(e) => tracing::warn!("tray update check failed: {e}"),
                    }
                });
            }
        })
        .build(app)?;

    Ok(())
}
