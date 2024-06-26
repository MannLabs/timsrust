use std::{
    fs,
    path::{Path, PathBuf},
};

pub mod vec_utils;

pub fn find_extension(
    path: impl AsRef<Path>,
    extension: &str,
) -> Option<PathBuf> {
    let extension_lower = extension.to_lowercase();
    for entry in fs::read_dir(&path).ok()? {
        if let Ok(entry) = entry {
            let file_path = entry.path();
            if let Some(file_name) =
                file_path.file_name().and_then(|name| name.to_str())
            {
                if file_name.to_lowercase().ends_with(&extension_lower) {
                    return Some(file_path);
                }
            }
        }
    }
    None
}
