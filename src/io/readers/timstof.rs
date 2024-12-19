use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum TimsTofFileType {
    #[cfg(feature = "minitdf")]
    MiniTDF,
    #[cfg(feature = "tdf")]
    TDF,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimsTofPath {
    path: PathBuf,
    file_type: TimsTofFileType,
}

impl TimsTofPath {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, TimsTofPathError> {
        let path = path.as_ref().canonicalize()?;
        #[cfg(feature = "tdf")]
        if tdf(&path).is_ok() & tdf_bin(&path).is_ok() {
            return Ok(Self {
                path,
                file_type: TimsTofFileType::TDF,
            });
        }
        #[cfg(feature = "minitdf")]
        if ms2_bin(&path).is_ok() & ms2_parquet(&path).is_ok() {
            return Ok(Self {
                path,
                file_type: TimsTofFileType::MiniTDF,
            });
        }
        match path.parent() {
            Some(parent) => match Self::new(parent) {
                Ok(result) => Ok(result),
                Err(_) => Err(TimsTofPathError::UnknownType(path)),
            },
            None => return Err(TimsTofPathError::UnknownType(path)),
        }
    }

    pub fn tdf(&self) -> Result<PathBuf, TimsTofPathError> {
        tdf(self)
    }

    pub fn tdf_bin(&self) -> Result<PathBuf, TimsTofPathError> {
        tdf_bin(self)
    }

    pub fn ms2_bin(&self) -> Result<PathBuf, TimsTofPathError> {
        ms2_bin(self)
    }

    pub fn ms2_parquet(&self) -> Result<PathBuf, TimsTofPathError> {
        ms2_parquet(self)
    }

    pub fn file_type(&self) -> TimsTofFileType {
        self.file_type
    }
}

fn tdf(path: impl AsRef<Path>) -> Result<PathBuf, TimsTofPathError> {
    find_extension(path, "analysis.tdf")
}

fn tdf_bin(path: impl AsRef<Path>) -> Result<PathBuf, TimsTofPathError> {
    find_extension(path, "analysis.tdf_bin")
}

fn ms2_bin(path: impl AsRef<Path>) -> Result<PathBuf, TimsTofPathError> {
    // match find_extension(path, "ms2.bin") {
    //     Ok(result) => Ok(result),
    //     Err(_) => find_extension(path, "ms2spectrum.bin"),
    // }
    // find_extension(path, "ms2.bin")
    find_extension(path, "ms2spectrum.bin")
}

fn ms2_parquet(path: impl AsRef<Path>) -> Result<PathBuf, TimsTofPathError> {
    // match find_extension(path, "ms2.parquet") {
    //     Ok(result) => Ok(result),
    //     Err(_) => find_extension(path, "ms2spectrum.parquet"),
    // }
    // find_extension(path, "ms2.parquet")
    find_extension(path, "ms2spectrum.parquet")
}

fn find_extension(
    path: impl AsRef<Path>,
    extension: &str,
) -> Result<PathBuf, TimsTofPathError> {
    let extension_lower = extension.to_lowercase();
    for entry in fs::read_dir(&path)? {
        if let Ok(entry) = entry {
            let file_path = entry.path();
            if let Some(file_name) =
                file_path.file_name().and_then(|name| name.to_str())
            {
                if file_name.to_lowercase().ends_with(&extension_lower) {
                    return Ok(file_path);
                }
            }
        }
    }
    Err(TimsTofPathError::Extension(
        extension.to_string(),
        path.as_ref().to_path_buf(),
    ))
}

impl AsRef<Path> for TimsTofPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

pub trait TimsTofPathLike: AsRef<Path> {
    fn to_timstof_path(&self) -> Result<TimsTofPath, TimsTofPathError>;
}

impl<T: AsRef<Path>> TimsTofPathLike for T {
    fn to_timstof_path(&self) -> Result<TimsTofPath, TimsTofPathError> {
        TimsTofPath::new(&self)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TimsTofPathError {
    #[error("Extension {0} not found for {1}")]
    Extension(String, PathBuf),
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("No valid type found for {0}")]
    UnknownType(PathBuf),
}
