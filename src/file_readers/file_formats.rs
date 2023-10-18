use std::{fs, path::PathBuf};

use crate::Error;

pub enum FileFormat {
    DFolder(PathBuf),
    MS2Folder(PathBuf)
}

impl FileFormat {
    pub fn parse(input: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let path: PathBuf = input.as_ref().to_path_buf();
        let extension: &str = path
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        let format = match extension {
            "d" => Self::DFolder(path),
            "ms2" => Self::MS2Folder(path),
            _ => {
                if let Some(path) = path.parent() {
                    // Only recurse if there is a valid parent section,
                    // otherwise we'll get a stack overflow
                    return Self::parse(path)
                }
                return Err(Error::UnknownFileFormat)
            },
        };
        if !format.is_valid() {
            Err(Error::UnknownFileFormat)
        } else {
            Ok(format)
        }
    }

    /// FileFormat is guaranteed to be `valid` if it is constructed
    fn is_valid(&self) -> bool {
        match &self {
            Self::DFolder(path) => folder_contains_extension(path, "tdf"),
            Self::MS2Folder(path) => folder_contains_extension(path, "parquet"),
        }
    }
}

fn folder_contains_extension(
    input: impl AsRef<std::path::Path>,
    extension: &str,
) -> bool {
    let folder_path: PathBuf = input.as_ref().to_path_buf();
    if !folder_path.is_dir() {
        return false;
    }
    if let Ok(entries) = fs::read_dir(folder_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(ext) = entry.path().extension() {
                    if ext == extension {
                        return true;
                    }
                }
            }
        }
    }
    false
}
