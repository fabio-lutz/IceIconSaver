use crate::store::IconEntry;
use std::collections::HashMap;
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider,
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::Common::{ITEMIDLIST, STRRET};
use windows::Win32::UI::Shell::{
    IEnumIDList, IFolderView, IFolderView2, IShellBrowser, IShellFolder, IShellWindows,
    FWF_AUTOARRANGE, SHGDN_FORPARSING, SHGDN_NORMAL, SID_STopLevelBrowser, SVGIO_ALLVIEW,
    SVSI_POSITIONITEM, SWC_DESKTOP, SWFO_NEEDDISPATCH, ShellWindows, StrRetToStrW, CSIDL_DESKTOP,
};
use windows::core::{Interface, VARIANT, PWSTR};

pub struct ComGuard;

impl ComGuard {
    pub fn init() -> Result<Self, String> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|e| format!("COM initialization failed: {e}"))?;
        }
        Ok(Self)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

fn find_desktop_folder_view() -> Result<IFolderView, String> {
    unsafe {
        let shell_windows: IShellWindows =
            CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)
                .map_err(|e| format!("Failed to create IShellWindows: {e}"))?;

        let mut hwnd = 0i32;
        let loc = VARIANT::from(CSIDL_DESKTOP as i32);
        let empty = VARIANT::default();
        let dispatch = shell_windows
            .FindWindowSW(&loc, &empty, SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)
            .map_err(|e| format!("FindWindowSW failed: {e}"))?;

        let provider: IServiceProvider = dispatch
            .cast()
            .map_err(|e| format!("IServiceProvider cast failed: {e}"))?;

        let browser: IShellBrowser = provider
            .QueryService(&SID_STopLevelBrowser)
            .map_err(|e| format!("QueryService(SID_STopLevelBrowser) failed: {e}"))?;

        let shell_view = browser
            .QueryActiveShellView()
            .map_err(|e| format!("QueryActiveShellView failed: {e}"))?;

        shell_view
            .cast::<IFolderView>()
            .map_err(|e| format!("IFolderView cast failed: {e}"))
    }
}

fn display_name(
    folder: &IShellFolder,
    pidl: *mut ITEMIDLIST,
    flags: windows::Win32::UI::Shell::SHGDNF,
) -> String {
    unsafe {
        let mut strret = STRRET::default();
        if folder.GetDisplayNameOf(pidl, flags, &mut strret).is_err() {
            return String::new();
        }

        let mut name = PWSTR::null();
        if StrRetToStrW(&mut strret, Some(pidl), &mut name).is_err() {
            return String::new();
        }

        let text = if name.is_null() {
            String::new()
        } else {
            name.to_string().unwrap_or_default()
        };

        if !name.is_null() {
            CoTaskMemFree(Some(name.as_ptr() as *const _));
        }

        text
    }
}

pub fn list_desktop_icons() -> Result<Vec<IconEntry>, String> {
    unsafe {
        let view = find_desktop_folder_view()?;
        let folder: IShellFolder = view
            .GetFolder()
            .map_err(|e| format!("GetFolder failed: {e}"))?;

        let enum_list: IEnumIDList = view
            .Items(SVGIO_ALLVIEW)
            .map_err(|e| format!("Items enumeration failed: {e}"))?;

        let mut icons = Vec::new();
        loop {
            let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
            let mut fetched = 0u32;
            let hr = enum_list.Next(std::slice::from_mut(&mut pidl), Some(&mut fetched));
            if hr.is_err() || fetched == 0 || pidl.is_null() {
                break;
            }

            let name = display_name(&folder, pidl, SHGDN_NORMAL);
            let id = display_name(&folder, pidl, SHGDN_FORPARSING);
            let point = view.GetItemPosition(pidl).unwrap_or(POINT { x: 0, y: 0 });

            icons.push(IconEntry {
                name,
                id,
                x: point.x,
                y: point.y,
            });

            CoTaskMemFree(Some(pidl as *const _));
        }

        Ok(icons)
    }
}

fn entry_matches(saved: &IconEntry, live_id: &str, live_name: &str) -> bool {
    if !saved.id.is_empty() {
        return saved.id.eq_ignore_ascii_case(live_id);
    }
    !saved.name.is_empty() && saved.name.eq_ignore_ascii_case(live_name)
}

/// Returns file entries that differ from the live desktop.
/// - Missing icons are included with `missing = true`
/// - Icons present at a different x/y are included with `missing = false`
/// - Icons already at the saved position are omitted
pub fn entries_needing_restore(icons: &[IconEntry]) -> Result<Vec<(IconEntry, bool)>, String> {
    let live = list_desktop_icons()?;
    Ok(icons
        .iter()
        .cloned()
        .filter_map(|icon| {
            match live
                .iter()
                .find(|live_icon| entry_matches(&icon, &live_icon.id, &live_icon.name))
            {
                Some(live_icon) if live_icon.x == icon.x && live_icon.y == icon.y => None,
                Some(_) => Some((icon, false)),
                None => Some((icon, true)),
            }
        })
        .collect())
}

pub fn entries_match(a: &IconEntry, b: &IconEntry) -> bool {
    entry_matches(a, &b.id, &b.name)
}

pub fn disable_auto_arrange_if_needed() -> Result<bool, String> {
    unsafe {
        let view = find_desktop_folder_view()?;
        let view2: IFolderView2 = view
            .cast()
            .map_err(|e| format!("IFolderView2 cast failed: {e}"))?;

        let flags = view2
            .GetCurrentFolderFlags()
            .map_err(|e| format!("GetCurrentFolderFlags failed: {e}"))?;

        let auto = (flags & FWF_AUTOARRANGE.0 as u32) != 0;
        if auto {
            view2
                .SetCurrentFolderFlags(FWF_AUTOARRANGE.0 as u32, 0)
                .map_err(|e| format!("SetCurrentFolderFlags failed: {e}"))?;
        }
        Ok(auto)
    }
}

#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub entry: IconEntry,
    pub status: String,
}

pub fn apply_positions(icons: &[(IconEntry, bool)]) -> Result<Vec<ApplyResult>, String> {
    unsafe {
        let view = find_desktop_folder_view()?;
        let folder: IShellFolder = view
            .GetFolder()
            .map_err(|e| format!("GetFolder failed: {e}"))?;

        let enum_list: IEnumIDList = view
            .Items(SVGIO_ALLVIEW)
            .map_err(|e| format!("Items enumeration failed: {e}"))?;

        let mut live_by_key: HashMap<String, *mut ITEMIDLIST> = HashMap::new();
        let mut owned_pidls: Vec<*mut ITEMIDLIST> = Vec::new();

        loop {
            let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
            let mut fetched = 0u32;
            let hr = enum_list.Next(std::slice::from_mut(&mut pidl), Some(&mut fetched));
            if hr.is_err() || fetched == 0 || pidl.is_null() {
                break;
            }

            let name = display_name(&folder, pidl, SHGDN_NORMAL);
            let id = display_name(&folder, pidl, SHGDN_FORPARSING);
            let key = if id.is_empty() {
                format!("name:{}", name.to_lowercase())
            } else {
                format!("id:{}", id.to_lowercase())
            };
            live_by_key.insert(key, pidl);
            owned_pidls.push(pidl);
        }

        let mut results = Vec::new();

        for (entry, missing) in icons {
            if *missing {
                results.push(ApplyResult {
                    entry: entry.clone(),
                    status: "Missing".to_string(),
                });
                continue;
            }

            let lookup_key = if !entry.id.is_empty() {
                format!("id:{}", entry.id.to_lowercase())
            } else {
                format!("name:{}", entry.name.to_lowercase())
            };

            let Some(pidl) = live_by_key.get(&lookup_key) else {
                results.push(ApplyResult {
                    entry: entry.clone(),
                    status: "Missing".to_string(),
                });
                continue;
            };

            let point = POINT {
                x: entry.x,
                y: entry.y,
            };
            let apidl = [*pidl as *const ITEMIDLIST];
            match view.SelectAndPositionItems(
                1,
                apidl.as_ptr(),
                Some(&point as *const POINT),
                SVSI_POSITIONITEM.0 as u32,
            ) {
                Ok(()) => results.push(ApplyResult {
                    entry: entry.clone(),
                    status: "Moved".to_string(),
                }),
                Err(e) => results.push(ApplyResult {
                    entry: entry.clone(),
                    status: format!("Error: {e}"),
                }),
            }
        }

        for pidl in owned_pidls {
            CoTaskMemFree(Some(pidl as *const _));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_list_desktop_icons() {
        let _guard = ComGuard::init().expect("COM init");
        let icons = list_desktop_icons().expect("list desktop icons");
        eprintln!("desktop icon count = {}", icons.len());
        for icon in icons.iter().take(5) {
            eprintln!(
                "{} | id={} | {},{}",
                icon.name, icon.id, icon.x, icon.y
            );
        }
    }
}
