use std::{fs, path::PathBuf};

use crate::{io::readers::frame_reader::FrameReader, ms_data::Frame};
use rayon::iter::ParallelIterator;

use super::common::sql_reader::SqlReader;
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

impl FileFormat {
    fn get_frame_reader(&self) -> FrameReader {
        let path = match &self {
            Self::DFolder(path) => path,
            Self::MS2Folder(path) => panic!(
                "Folder {:} is not frame readable",
                path.to_str().unwrap_or_default().to_string()
            ),
        };
        let frame_reader: FrameReader = FrameReader::new(&path);
        frame_reader
    }

    pub fn read_single_frame(&self, index: usize) -> Frame {
        self.get_frame_reader().get(index)
    }

    pub fn read_all_frames(&self) -> Vec<Frame> {
        self.get_frame_reader().parallel_filter(|_| true).collect()
    }

    pub fn read_all_ms1_frames(&self) -> Vec<Frame> {
        self.get_frame_reader()
            .parallel_filter(|x| x.msms_type == 0)
            .collect()
    }

    pub fn read_all_ms2_frames(&self) -> Vec<Frame> {
        self.get_frame_reader()
            .parallel_filter(|x| x.msms_type != 0)
            .collect()
    }
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
