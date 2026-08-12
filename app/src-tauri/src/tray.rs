use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App,
};

// Windows renders a blank tray square when no icon is provided, so embed the 32x32 PNG at compile time.
const TRAY_ICON: &[u8] = include_bytes!("../icons/32x32.png");

pub fn create_tray(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_i])?;

    let _tray = TrayIconBuilder::new()
        .icon(Image::from_bytes(TRAY_ICON)?)
        .menu(&menu)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .build(app)?;

    Ok(())
}
