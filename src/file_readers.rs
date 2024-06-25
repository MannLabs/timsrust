mod dda_reader;
mod mini_tdf_reader;

use std::{fs, path::PathBuf};

use crate::{io::readers::FrameReader, Error};

use dda_reader::DDASpectrumReader;
use mini_tdf_reader::MiniTDFReader;
use rayon::iter::ParallelIterator;
use {
    // self::file_formats::FileFormat,
    crate::ms_data::{Frame, Spectrum},
};

/// A reader to read [frames](crate::ms_data::Frame) and [spectra](crate::ms_data::Spectrum).
pub struct FileReader {
    format: FileFormat,
}

///NOTE: The functions to read a single frame or spectrum are not optimized.
/// In case many frames or spectra are required, it is best to use
/// any of the functions that directly return a `Vec`.
impl FileReader {
    pub fn new<T: AsRef<std::path::Path>>(path_name: T) -> Result<Self, Error> {
        let format: FileFormat = FileFormat::parse(path_name)?;
        Ok(Self { format })
    }

    pub fn read_single_frame(&self, index: usize) -> Frame {
        self.format.read_single_frame(index)
    }

    pub fn read_all_frames(&self) -> Vec<Frame> {
        self.format.read_all_frames()
    }

    /// NOTE: The returned vec contains all frames to not disrupt indexing.
    /// MS2 frames are set to unknown and not read.
    pub fn read_all_ms1_frames(&self) -> Vec<Frame> {
        self.format.read_all_ms1_frames()
    }

    /// NOTE: The returned vec contains all frames to not disrupt indexing.
    /// MS1 frames are set to unknown and not read.
    pub fn read_all_ms2_frames(&self) -> Vec<Frame> {
        self.format.read_all_ms2_frames()
    }

    pub fn read_single_spectrum(&self, index: usize) -> Spectrum {
        self.format.read_single_spectrum(index)
    }

    ///NOTE: ddaPASEF MS2 spectra are automatically calibrated with
    /// all unfragmented precursor signals.
    /// Hence, reading spectra individually through `read_single_spectrum`
    /// might yield slightly different mz values.
    pub fn read_all_spectra(&self) -> Vec<Spectrum> {
        self.format.read_all_spectra()
    }
}

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

    pub fn read_single_spectrum(&self, index: usize) -> Spectrum {
        match &self {
            Self::DFolder(path) => DDASpectrumReader::new(
                path.to_str().unwrap_or_default().to_string(),
            )
            .read_single_spectrum(index),
            Self::MS2Folder(path) => MiniTDFReader::new(
                path.to_str().unwrap_or_default().to_string(),
            )
            .read_single_spectrum(index),
        }
    }

    pub fn read_all_spectra(&self) -> Vec<Spectrum> {
        match &self {
            Self::DFolder(path) => DDASpectrumReader::new(
                path.to_str().unwrap_or_default().to_string(),
            )
            .read_all_spectra(),
            Self::MS2Folder(path) => MiniTDFReader::new(
                path.to_str().unwrap_or_default().to_string(),
            )
            .read_all_spectra(),
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
