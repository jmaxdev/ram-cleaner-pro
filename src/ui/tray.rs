use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayManager {
    pub tray_icon: TrayIcon,
    pub item_purge_now: MenuItem,
    pub item_toggle_gui: MenuItem,
    pub item_toggle_auto: MenuItem,
    pub item_quit: MenuItem,
}

impl TrayManager {
    pub fn new() -> Result<Self, String> {
        let menu = Menu::new();

        let item_purge_now = MenuItem::new("Purge Memory Now", true, None);
        let item_toggle_gui = MenuItem::new("Open GUI Dashboard", true, None);
        let item_toggle_auto = MenuItem::new("Auto-Purge (Enabled)", true, None);
        let item_quit = MenuItem::new("Exit", true, None);

        let _ = menu.append(&item_purge_now);
        let _ = menu.append(&item_toggle_gui);
        let _ = menu.append(&item_toggle_auto);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&item_quit);

        let icon = create_default_icon()?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("RAM Purger Pro - Monitoring RAM...")
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            tray_icon,
            item_purge_now,
            item_toggle_gui,
            item_toggle_auto,
            item_quit,
        })
    }

    pub fn update_tooltip(&self, text: &str) {
        let _ = self.tray_icon.set_tooltip(Some(text));
    }
}

fn create_default_icon() -> Result<Icon, String> {
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - 16.0) / 14.0;
            let dy = (y as f32 - 16.0) / 14.0;
            let r2 = dx * dx + dy * dy;

            if r2 <= 1.0 && r2 >= 0.65 {
                rgba.extend_from_slice(&[0, 210, 255, 255]);
            } else if r2 < 0.65 {
                rgba.extend_from_slice(&[15, 23, 42, 240]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    Icon::from_rgba(rgba, width, height).map_err(|e| e.to_string())
}
