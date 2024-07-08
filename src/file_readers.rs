use std::{fs, path::PathBuf};

use crate::io::readers::file_readers::sql_reader::frames::SqlFrame;
use crate::io::readers::SpectrumReader;
use crate::{io::readers::FrameReader, Error};

use crate::ms_data::{Frame, Spectrum};
use rayon::iter::ParallelIterator;

/// A reader to read [frames](crate::ms_data::Frame) and [spectra](crate::ms_data::Spectrum).
pub struct FileReader {
    frame_reader: Option<FrameReader>,
    spectrum_reader: Option<SpectrumReader>,
}

impl FileReader {
    // TODO refactor out
    // TODO proper error handling
    // TODO update docs
    pub fn new<T: AsRef<std::path::Path>>(path_name: T) -> Result<Self, Error> {
        let format: FileFormat = FileFormat::parse(path_name)?;
        let frame_reader = match &format {
            FileFormat::DFolder(path) => Some(FrameReader::new(&path)),
            FileFormat::MS2Folder(_) => None,
        };
        let spectrum_reader = match &format {
            FileFormat::DFolder(path) => {
                let reader = SpectrumReader::new(path);
                // reader.calibrate();
                Some(reader)
            },
            FileFormat::MS2Folder(path) => Some(SpectrumReader::new(path)),
        };
        Ok(Self {
            frame_reader,
            spectrum_reader,
        })
    }

    pub fn read_single_frame(&self, index: usize) -> Frame {
        self.frame_reader.as_ref().unwrap().get(index)
    }

    fn read_multiple_frames<'a, F: Fn(&SqlFrame) -> bool + Sync + Send + 'a>(
        &self,
        predicate: F,
    ) -> Vec<Frame> {
        self.frame_reader
            .as_ref()
            .unwrap()
            .parallel_filter(|x| predicate(x))
            .collect()
    }

    pub fn read_all_frames(&self) -> Vec<Frame> {
        self.read_multiple_frames(|_| true)
    }

    pub fn read_all_ms1_frames(&self) -> Vec<Frame> {
        self.read_multiple_frames(|x| x.msms_type == 0)
    }

    pub fn read_all_ms2_frames(&self) -> Vec<Frame> {
        self.read_multiple_frames(|x| x.msms_type != 0)
    }

    pub fn read_single_spectrum(&self, index: usize) -> Spectrum {
        self.spectrum_reader.as_ref().unwrap().get(index)
    }

    pub fn read_all_spectra(&self) -> Vec<Spectrum> {
        self.spectrum_reader.as_ref().unwrap().get_all()
    }
}

pub enum FileFormat {
    DFolder(PathBuf),
    MS2Folder(PathBuf),
}

impl FileFormat {
    // TODO make into proper struct
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
