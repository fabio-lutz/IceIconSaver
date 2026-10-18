use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const FILE_EXTENSION: &str = "deskicons";
pub const FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconEntry {
    pub name: String,
    pub id: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconLayoutFile {
    pub version: u32,
    pub icons: Vec<IconEntry>,
}

pub fn storage_dir() -> Result<PathBuf, String> {
    let profile = std::env::var("USERPROFILE")
        .map_err(|_| "USERPROFILE environment variable is not set".to_string())?;
    let dir = PathBuf::from(profile).join(".iceiconsaver");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    Ok(dir)
}

pub fn ensure_deskicons_extension(path: PathBuf) -> PathBuf {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case(FILE_EXTENSION) => path,
        _ => path.with_extension(FILE_EXTENSION),
    }
}

pub fn save_layout(path: &Path, icons: &[IconEntry]) -> Result<(), String> {
    let file = IconLayoutFile {
        version: FILE_VERSION,
        icons: icons.to_vec(),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| format!("Failed to serialize layout: {e}"))?;
    fs::write(path, json).map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

pub fn load_layout(path: &Path) -> Result<Vec<IconEntry>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let file: IconLayoutFile = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;
    if file.version != FILE_VERSION {
        return Err(format!(
            "Unsupported file version {} (expected {FILE_VERSION})",
            file.version
        ));
    }
    Ok(file.icons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn appends_deskicons_extension() {
        let path = ensure_deskicons_extension(PathBuf::from(r"C:\temp\layout"));
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("deskicons"));
    }

    #[test]
    fn keeps_existing_deskicons_extension() {
        let path = ensure_deskicons_extension(PathBuf::from(r"C:\temp\layout.deskicons"));
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("layout.deskicons"));
    }
}
