use std::{borrow::Cow, io::Cursor};
use timsrust_core::FrameIons;
use timsrust_core::io::formats::binary::{BinaryError, BinaryReader};
use timsrust_core::utils::reader::Reader;
use zstd::decode_all;

use crate::{TDFPath, TDFPathError, TDFPathLike};

const U32_SIZE: usize = std::mem::size_of::<u32>();
const HEADER_SIZE: usize = 2;
const BLOB_TYPE_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Debug)]
pub(crate) struct TdfBlobReader {
    bin_file_reader: TdfBinFileReader,
}

impl TdfBlobReader {
    pub(crate) fn new(
        path: Result<TDFPath, TDFPathError>,
    ) -> Result<Self, TdfBlobReaderError> {
        let bin_file_reader = TdfBinFileReader::new(path?)?;
        let reader = Self { bin_file_reader };
        Ok(reader)
    }

    fn read_blob_at_offset(
        &self,
        offset: usize,
    ) -> Result<TdfBlob, TdfBlobReaderError> {
        let offset = self.bin_file_reader.global_file_offset + offset;
        let byte_count = self
            .bin_file_reader
            .get_byte_count(offset)
            .ok_or(TdfBlobReaderError::InvalidOffset(offset))?;
        let data = self
            .bin_file_reader
            .get_data(offset, byte_count)
            .ok_or(TdfBlobReaderError::CorruptData)?;
        if data.is_empty() {
            return Err(TdfBlobReaderError::EmptyData);
        }
        let bytes = decode_all(Cursor::new(data))
            .map_err(|_| TdfBlobReaderError::Decompression)?;
        let blob = TdfBlob::new(Cow::Owned(bytes))?;
        Ok(blob)
    }
}

#[derive(Debug)]
struct TdfBinFileReader {
    binary_file: BinaryReader,
    global_file_offset: usize,
}

impl TdfBinFileReader {
    // TODO parse compression1
    fn new(path: impl TDFPathLike) -> Result<Self, TdfBlobReaderError> {
        let path = path.to_timstof_path()?;
        let bin_path = path.tdf_bin();
        let binary_file = BinaryReader::from(bin_path.as_ref())?;
        let reader = Self {
            binary_file,
            global_file_offset: 0,
        };
        Ok(reader)
    }

    fn get_byte_count(&self, offset: usize) -> Option<usize> {
        let start = offset;
        let end = start + U32_SIZE;
        let raw_byte_count = self.binary_file.read_range(start..end).ok()?;
        let byte_count =
            u32::from_le_bytes(raw_byte_count.try_into().ok()?) as usize;
        Some(byte_count)
    }

    fn get_data(&self, offset: usize, byte_count: usize) -> Option<Vec<u8>> {
        let start = offset + HEADER_SIZE * U32_SIZE;
        let end = offset + byte_count;
        self.binary_file.read_range(start..end).ok()
    }
}

#[allow(private_interfaces)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum TdfBlobReaderError {
    #[error("{0}")]
    TdfBlob(#[from] TdfBlobError),
    #[error("No binary data")]
    EmptyData,
    #[error("Data is corrupt")]
    CorruptData,
    #[error("Decompression fails")]
    Decompression,
    #[error("Invalid offset {0}")]
    InvalidOffset(usize),
    #[error("{0}")]
    TDFPathError(#[from] TDFPathError),
    #[error("{0}")]
    FileError(#[from] BinaryError),
    #[error("Corrupt frame data")]
    CorruptFrame,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct TdfBlob {
    bytes: Vec<u8>,
}

impl TdfBlob {
    fn concatenate_bytes(b1: u8, b2: u8, b3: u8, b4: u8) -> u32 {
        b1 as u32
            | ((b2 as u32) << 8)
            | ((b3 as u32) << 16)
            | ((b4 as u32) << 24)
    }

    fn len(&self) -> usize {
        self.bytes.len() / BLOB_TYPE_SIZE
    }

    fn new(bytes: std::borrow::Cow<[u8]>) -> Result<Self, TdfBlobError> {
        if !bytes.len().is_multiple_of(BLOB_TYPE_SIZE) {
            Err(TdfBlobError(bytes.len()))
        } else {
            Ok(Self {
                bytes: bytes.into_owned(),
            })
        }
    }

    fn get(&self, index: usize) -> Option<u32> {
        if index >= self.len() {
            None
        } else {
            Some(Self::concatenate_bytes(
                self.bytes[index],
                self.bytes[index + self.len()],
                self.bytes[index + 2 * self.len()],
                self.bytes[index + 3 * self.len()],
            ))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Length {0} is not a multiple of {BLOB_TYPE_SIZE}")]
struct TdfBlobError(usize);

impl Reader<FrameIons> for TdfBlobReader {
    type Error = TdfBlobReaderError;

    fn get(&self, index: usize) -> Result<FrameIons, Self::Error> {
        // let blob = match self.read_blob_at_offset(index) {
        //     Ok(blob) => blob,
        //     Err(e) => match e {
        //         TdfBlobReaderError::EmptyData => {
        //             return Ok(FrameIons::default());
        //         },
        //         _ => return Err(e),
        //     },
        // };
        let blob = self.read_blob_at_offset(index)?;
        let scan_count: usize =
            blob.get(0).expect("Blob cannot be empty") as usize;
        let peak_count: usize = (blob.len() - scan_count) / 2;
        let scan_offsets = read_scan_offsets(scan_count, peak_count, &blob)?;
        let intensities = read_intensities(scan_count, peak_count, &blob)?;
        let tof_indices =
            read_tof_indices(scan_count, peak_count, &blob, &scan_offsets)?;
        let frame_ions = FrameIons::new(
            scan_offsets,
            tof_indices.iter().map(|&x| x.try_into().unwrap()).collect(),
            intensities.iter().map(|&x| x.try_into().unwrap()).collect(),
        );
        Ok(frame_ions)
    }
}

fn read_scan_offsets(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Result<Vec<usize>, TdfBlobReaderError> {
    let mut scan_offsets: Vec<usize> = Vec::with_capacity(scan_count + 1);
    scan_offsets.push(0);
    for scan_index in 0..scan_count - 1 {
        let index = scan_index + 1;
        let scan_size: usize =
            (blob.get(index).ok_or(TdfBlobReaderError::CorruptFrame)? / 2)
                as usize;
        scan_offsets.push(scan_offsets[scan_index] + scan_size);
    }
    scan_offsets.push(peak_count);
    Ok(scan_offsets)
}

fn read_intensities(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Result<Vec<u32>, TdfBlobReaderError> {
    let mut intensities: Vec<u32> = Vec::with_capacity(peak_count);
    for peak_index in 0..peak_count {
        let index: usize = scan_count + 1 + 2 * peak_index;
        intensities
            .push(blob.get(index).ok_or(TdfBlobReaderError::CorruptFrame)?);
    }
    Ok(intensities)
}

fn read_tof_indices(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    scan_offsets: &[usize],
) -> Result<Vec<u32>, TdfBlobReaderError> {
    let mut tof_indices: Vec<u32> = Vec::with_capacity(peak_count);
    for scan_index in 0..scan_count {
        let start_offset: usize = scan_offsets[scan_index];
        let end_offset: usize = scan_offsets[scan_index + 1];
        let mut current_sum: u32 = 0;
        for peak_index in start_offset..end_offset {
            let index = scan_count + 2 * peak_index;
            let tof_index: u32 =
                blob.get(index).ok_or(TdfBlobReaderError::CorruptFrame)?;
            current_sum += tof_index;
            tof_indices.push(current_sum - 1);
        }
    }
    Ok(tof_indices)
}
