use crate::store;
use native_windows_gui as nwg;
use std::path::PathBuf;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSubMenu, ModifyMenuW, MF_BYCOMMAND, MF_BYPOSITION, MF_POPUP, MF_STRING,
};
use windows::core::PCWSTR;

pub const LOCALE_EN: &str = "en";
pub const LOCALE_PT_BR: &str = "pt-BR";

fn locale_file_path() -> Result<PathBuf, String> {
    Ok(store::storage_dir()?.join("locale"))
}

pub fn load_saved_locale() -> Option<String> {
    let path = locale_file_path().ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    match trimmed {
        LOCALE_EN | LOCALE_PT_BR => Some(trimmed.to_string()),
        _ => None,
    }
}

pub fn save_locale(locale: &str) -> Result<(), String> {
    let path = locale_file_path()?;
    std::fs::write(&path, locale).map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

pub fn resolve_startup_locale() -> String {
    if let Some(saved) = load_saved_locale() {
        return saved;
    }

    let user = nwg::Locale::user();
    let name = user.name().trim_matches('\0').to_lowercase();
    if name.starts_with("pt") {
        LOCALE_PT_BR.to_string()
    } else {
        LOCALE_EN.to_string()
    }
}

pub fn apply_locale(locale: &str) {
    rust_i18n::set_locale(locale);
}

pub fn current_locale() -> String {
    rust_i18n::locale().to_string()
}

pub fn display_status(code: &str) -> String {
    match code {
        "SAVED" => rust_i18n::t!("status_saved").to_string(),
        "UPDATED" => rust_i18n::t!("status_updated").to_string(),
        "Missing" => rust_i18n::t!("status_missing").to_string(),
        other => other.to_string(),
    }
}

fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Updates the label of a menu bar popup (File, Help, Language).
pub fn set_menu_bar_text(menu: &nwg::Menu, text: &str) {
    let Some((parent, submenu)) = menu.handle.hmenu() else {
        return;
    };
    // Parent of a top-level menu is the window HWND; menu bar is GetMenu(hwnd).
    // For nested menus, parent is the parent HMENU.
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetMenu, GetMenuItemCount};

        let wide = to_wide(text);
        let parent_hwnd = HWND(parent as *mut _);

        // Top-level: parent is HWND of the window.
        let menubar = GetMenu(parent_hwnd);
        if !menubar.is_invalid() {
            let count = GetMenuItemCount(menubar);
            for i in 0..count {
                let sub = GetSubMenu(menubar, i);
                if sub.0 == submenu as *mut _ {
                    let _ = ModifyMenuW(
                        menubar,
                        i as u32,
                        MF_BYPOSITION | MF_STRING | MF_POPUP,
                        submenu as usize,
                        PCWSTR(wide.as_ptr()),
                    );
                    return;
                }
            }
        }

        // Nested submenu item: parent is HMENU.
        use windows::Win32::UI::WindowsAndMessaging::HMENU;
        let parent_menu = HMENU(parent as *mut _);
        let count = GetMenuItemCount(parent_menu);
        for i in 0..count {
            let sub = GetSubMenu(parent_menu, i);
            if sub.0 == submenu as *mut _ {
                let _ = ModifyMenuW(
                    parent_menu,
                    i as u32,
                    MF_BYPOSITION | MF_STRING | MF_POPUP,
                    submenu as usize,
                    PCWSTR(wide.as_ptr()),
                );
                return;
            }
        }
    }
}

pub fn set_menu_item_text(item: &nwg::MenuItem, text: &str) {
    let Some((parent, id)) = item.handle.hmenu_item() else {
        return;
    };
    let wide = to_wide(text);
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::HMENU;
        let _ = ModifyMenuW(
            HMENU(parent as *mut _),
            id,
            MF_BYCOMMAND | MF_STRING,
            id as usize,
            PCWSTR(wide.as_ptr()),
        );
    }
}
