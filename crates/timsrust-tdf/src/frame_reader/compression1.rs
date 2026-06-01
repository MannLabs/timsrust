use lzf::decompress as lzf_decompress;
use timsrust_core::io::formats::binary::{BinaryError, BinaryReader};
use timsrust_core::utils::reader::Reader;
use timsrust_core::{
    FrameIons,
    // FrameIonsReader,
    // blobs::{Blob, BlobReader},
};

use crate::{TDFPath, TDFPathError, TDFPathLike};

const U32_SIZE: usize = std::mem::size_of::<u32>();
const HEADER_SIZE: usize = 2;
const BLOB_TYPE_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Debug)]
pub(crate) struct TdfBlobReaderCompression1 {
    bin_file_reader: TdfBinFileReader,
    max_peaks_per_scan: usize,
}

impl TdfBlobReaderCompression1 {
    /// Get a TDF blob compressed with version 1
    /// Basically a reimplementation of the alphatims implementation
    /// Returns the uncompressed data compatible
    /// * scan_count: 4 bytes
    /// * scan_indices: (scan_count) * 4 bytes
    /// * scan: remaining bytes
    ///
    /// # Arguments
    /// * `offset` - The offset of the blob in the binary file
    /// * `max_peaks_per_scan` - The maximum number of peaks per scan
    /// * `data` - The compressed data
    /// * `max_peaks_per_scan` - The maximum number of peaks per scan from the metadata
    fn decompress_v1(
        &self,
        offset: usize,
        data: &[u8],
        max_peaks_per_scan: usize,
    ) -> Result<Vec<u8>, TdfBlobReaderErrorCompression1> {
        let scan_count = self
            .bin_file_reader
            .get_scan_count(offset)
            .ok_or(TdfBlobReaderErrorCompression1::NoScanCount)?;
        let max_peak_count = max_peaks_per_scan * 2;
        // if scan_count > 1000 {
        //     return Err(TdfBlobReaderErrorCompression1::ScanOffsetError);
        // }
        // if scan_count > 1000 {
        //     dbg!(offset, scan_count);
        // }
        let scan_offsets = data[..(scan_count + 1) * U32_SIZE]
            .chunks_exact(U32_SIZE)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
            .map(|x| x as usize - HEADER_SIZE * U32_SIZE)
            .collect::<Vec<usize>>();
        let mut tdf_bytes = vec![];
        let mut last_offset = scan_count as u32 + 1;
        let mut scan_bytes = last_offset.to_le_bytes().to_vec();
        for scan_index in 0..scan_count {
            let start = scan_offsets[scan_index];
            let end = scan_offsets[scan_index + 1];
            if start == end {
                scan_bytes.extend(last_offset.to_le_bytes());
                continue;
            }
            let decompressed_bytes = match lzf_decompress(
                &data[start..end],
                max_peak_count * U32_SIZE,
            ) {
                Ok(bytes) => bytes,
                Err(_) => {
                    return Err(TdfBlobReaderErrorCompression1::Decompression);
                },
            };
            if decompressed_bytes.len() % U32_SIZE != 0 {
                return Err(TdfBlobReaderErrorCompression1::CorruptData);
            }
            last_offset += decompressed_bytes.len() as u32 / U32_SIZE as u32;
            scan_bytes.extend(last_offset.to_le_bytes());
            tdf_bytes.extend(decompressed_bytes);
        }
        let mut blob_bytes = scan_bytes;
        blob_bytes.extend(tdf_bytes);
        Ok(blob_bytes)
    }

    pub(crate) fn set_max_peaks_per_scan(&mut self, max_peaks_per_scan: usize) {
        self.max_peaks_per_scan = max_peaks_per_scan;
    }

    pub(crate) fn new(
        path: Result<TDFPath, TDFPathError>,
    ) -> Result<Self, TdfBlobReaderErrorCompression1> {
        let bin_file_reader = TdfBinFileReader::new(path?)?;
        let reader = Self {
            bin_file_reader,
            max_peaks_per_scan: 0,
        };
        Ok(reader)
    }

    fn read_blob_at_offset(
        &self,
        offset: usize,
    ) -> Result<TdfBlobCompression1, TdfBlobReaderErrorCompression1> {
        let offset = self.bin_file_reader.global_file_offset + offset;
        let byte_count = self
            .bin_file_reader
            .get_byte_count(offset)
            .ok_or(TdfBlobReaderErrorCompression1::InvalidOffset(offset))?;
        let data = self
            .bin_file_reader
            .get_data(offset, byte_count)
            .ok_or(TdfBlobReaderErrorCompression1::CorruptData)?;
        if data.is_empty() {
            return Err(TdfBlobReaderErrorCompression1::EmptyData);
        }
        let blob = {
            let bytes =
                self.decompress_v1(offset, &data, self.max_peaks_per_scan)?;
            TdfBlobCompression1::new(std::borrow::Cow::Owned(bytes))?
        };
        Ok(blob)
    }
}

#[derive(Debug)]
struct TdfBinFileReader {
    binary_file: BinaryReader,
    global_file_offset: usize,
}

impl TdfBinFileReader {
    /// Get scan count, second 4 bytes of the blob
    ///
    fn get_scan_count(&self, offset: usize) -> Option<usize> {
        let start = offset + U32_SIZE;
        let end = start + U32_SIZE;
        let raw_scan_count = self.binary_file.read_range(start..end).ok()?;
        let scan_count =
            u32::from_le_bytes(raw_scan_count.try_into().ok()?) as usize;
        Some(scan_count)
    }

    fn new(
        path: impl TDFPathLike,
    ) -> Result<Self, TdfBlobReaderErrorCompression1> {
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
pub(crate) enum TdfBlobReaderErrorCompression1 {
    #[error("{0}")]
    TdfBlobCompression1(#[from] TdfBlobError),
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
    #[error("No scan count found")]
    NoScanCount,
    #[error("Corrupt frame")]
    CorruptFrame,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct TdfBlobCompression1 {
    bytes: Vec<u8>,
}

impl TdfBlobCompression1 {
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
            let index = index * BLOB_TYPE_SIZE;
            Some(Self::concatenate_bytes(
                self.bytes[index],
                self.bytes[index + 1],
                self.bytes[index + 2],
                self.bytes[index + 3],
            ))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Length {0} is not a multiple of {BLOB_TYPE_SIZE}")]
struct TdfBlobError(usize);

impl Reader<FrameIons> for TdfBlobReaderCompression1 {
    type Error = TdfBlobReaderErrorCompression1;

    fn get(&self, index: usize) -> Result<FrameIons, Self::Error> {
        let blob = self.read_blob_at_offset(index)?;
        let mut scan_offsets = vec![0];
        let mut intensities = vec![];
        let mut tof_indices = vec![];
        let mut start: usize = blob
            .get(0)
            .ok_or(TdfBlobReaderErrorCompression1::CorruptFrame)?
            as usize;
        let scan_count = start - 1;
        for i in 0..scan_count {
            let end = blob
                .get(i + 1)
                .ok_or(TdfBlobReaderErrorCompression1::CorruptFrame)?
                as usize;
            let mut tof_index = 0;
            for j in start..end {
                let value = blob
                    .get(j)
                    .ok_or(TdfBlobReaderErrorCompression1::CorruptFrame)?;
                let value = i32::from_le_bytes(value.to_le_bytes());
                if value > 0 {
                    intensities.push(value as u32);
                    tof_index -= 1;
                    tof_indices.push(-tof_index as u32);
                } else {
                    tof_index += value + 1;
                }
            }
            start = end;
            scan_offsets.push(intensities.len());
        }
        let frame_ions = FrameIons::new(
            scan_offsets,
            tof_indices.iter().map(|&x| x.try_into().unwrap()).collect(),
            intensities.iter().map(|&x| x.try_into().unwrap()).collect(),
        );
        Ok(frame_ions)
    }
}
