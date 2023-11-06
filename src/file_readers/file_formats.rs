use std::{fs, path::PathBuf};

pub enum FileFormat {
    DFolder(PathBuf),
    MS2Folder(PathBuf),
}

impl FileFormat {
    pub fn parse(
        input: impl AsRef<std::path::Path>,
    ) -> Result<Self, FileFormatError> {
        let path: PathBuf = input.as_ref().to_path_buf();
        if !path.exists() {
            return Err(FileFormatError::DirectoryDoesNotExist);
        }
        let extension: &str = path
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        let format = match extension {
            "d" => Self::DFolder(path),
            _ => Self::MS2Folder(path),
        };
        format.is_valid()?;
        Ok(format)
    }

    /// FileFormat is guaranteed to be `valid` if it is constructed
    fn is_valid(&self) -> Result<(), FileFormatError> {
        match &self {
            Self::DFolder(path) => {
                if !folder_contains_extension(path, "tdf_bin") {
                    return Err(FileFormatError::BinaryFilesAreMissing);
                }
                if !folder_contains_extension(path, "tdf") {
                    return Err(FileFormatError::MetadataFilesAreMissing);
                }
            },
            Self::MS2Folder(path) => {
                if !folder_contains_extension(path, "bin") {
                    return Err(FileFormatError::BinaryFilesAreMissing);
                }
                if !folder_contains_extension(path, "parquet") {
                    return Err(FileFormatError::MetadataFilesAreMissing);
                }
            },
        }
        Ok(())
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

#[derive(thiserror::Error, Debug)]
pub enum FileFormatError {
    #[error("DirectoryDoesNotExist")]
    DirectoryDoesNotExist,
    #[error("NoParentWithBrukerExtension")]
    NoParentWithBrukerExtension,
    #[error("BinaryFilesAreMissing")]
    BinaryFilesAreMissing,
    #[error("MetadataFilesAreMissing")]
    MetadataFilesAreMissing,
}
