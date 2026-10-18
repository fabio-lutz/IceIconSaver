use crate::store;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SetWindowPlacement, WINDOWPLACEMENT, SW_SHOWMAXIMIZED, SW_SHOWNORMAL,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

fn window_state_path() -> Result<PathBuf, String> {
    Ok(store::storage_dir()?.join("window.json"))
}

pub fn load() -> Option<WindowState> {
    let path = window_state_path().ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(state: &WindowState) -> Result<(), String> {
    let path = window_state_path()?;
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize window state: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

pub fn read_from_hwnd(hwnd: HWND) -> Result<WindowState, String> {
    unsafe {
        let mut placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        GetWindowPlacement(hwnd, &mut placement)
            .map_err(|e| format!("GetWindowPlacement failed: {e}"))?;

        let rect = placement.rcNormalPosition;
        Ok(WindowState {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
            maximized: placement.showCmd == SW_SHOWMAXIMIZED.0 as u32,
        })
    }
}

pub fn apply_to_hwnd(hwnd: HWND, state: &WindowState) -> Result<(), String> {
    if state.width <= 0 || state.height <= 0 {
        return Err("Invalid saved window size".to_string());
    }

    unsafe {
        let mut placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        GetWindowPlacement(hwnd, &mut placement)
            .map_err(|e| format!("GetWindowPlacement failed: {e}"))?;

        placement.rcNormalPosition = RECT {
            left: state.x,
            top: state.y,
            right: state.x + state.width,
            bottom: state.y + state.height,
        };
        placement.showCmd = if state.maximized {
            SW_SHOWMAXIMIZED.0 as u32
        } else {
            SW_SHOWNORMAL.0 as u32
        };

        SetWindowPlacement(hwnd, &placement)
            .map_err(|e| format!("SetWindowPlacement failed: {e}"))?;
    }
    Ok(())
}

pub fn hwnd_from_nwg(window: &native_windows_gui::Window) -> Option<HWND> {
    window
        .handle
        .hwnd()
        .map(|hwnd| HWND(hwnd as *mut core::ffi::c_void))
}
