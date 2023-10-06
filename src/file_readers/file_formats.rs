use std::{fs, path::PathBuf};

pub enum FileFormat {
    DFolder(PathBuf),
    MS2Folder(PathBuf),
    Unknown(PathBuf),
}

impl FileFormat {
    pub fn parse(input: impl AsRef<std::path::Path>) -> Self {
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
                let parent_path: &std::path::Path =
                    path.parent().unwrap_or("".as_ref());
                Self::parse(parent_path)
            },
        };
        if !format.is_valid() {
            let path: PathBuf = input.as_ref().to_path_buf();
            Self::Unknown(path)
        } else {
            format
        }
    }

    pub fn is_valid(&self) -> bool {
        let result: bool = match &self {
            Self::DFolder(path) => folder_contains_extension(path, "tdf"),
            Self::MS2Folder(path) => folder_contains_extension(path, "parquet"),
            Self::Unknown(_) => false,
        };
        result
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
