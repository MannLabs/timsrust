use std::io::Cursor;
use timsrust_core::io::formats::binary::{BinaryError, BinaryReader};
use zstd::decode_all;

use crate::timstof::{MiniTDFPath, MiniTDFPathError};

const U32_SIZE: usize = std::mem::size_of::<u32>();
const HEADER_SIZE: usize = 2;
const BLOB_TYPE_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Debug)]
pub(crate) struct TdfBlobReader {
    bin_file_reader: TdfBinFileReader,
}

impl TdfBlobReader {
    fn new(path: &MiniTDFPath) -> Result<Self, TdfBlobReaderError> {
        let bin_file_reader = TdfBinFileReader::new(path)?;
        let reader = Self { bin_file_reader };
        Ok(reader)
    }

    fn get(&self, offset: usize) -> Result<TdfBlob, TdfBlobReaderError> {
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
        let blob = TdfBlob::new(bytes)?;
        Ok(blob)
    }
}

#[derive(Debug)]
struct TdfBinFileReader {
    binary_file: BinaryReader,
    global_file_offset: usize,
}

impl TdfBinFileReader {
    fn new(path: &MiniTDFPath) -> Result<Self, TdfBlobReaderError> {
        let bin_path = path.ms2_bin();
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

#[derive(Debug)]
pub(crate) struct IndexedTdfBlobReader {
    blob_reader: TdfBlobReader,
    binary_offsets: Vec<usize>,
}

impl IndexedTdfBlobReader {
    pub(crate) fn new(
        path: &MiniTDFPath,
        binary_offsets: Vec<usize>,
    ) -> Result<Self, IndexedTdfBlobReaderError> {
        let blob_reader = TdfBlobReader::new(path)?;
        let reader = Self {
            binary_offsets,
            blob_reader,
        };
        Ok(reader)
    }

    pub(crate) fn get(
        &self,
        index: usize,
    ) -> Result<TdfBlob, IndexedTdfBlobReaderError> {
        let offset = *self
            .binary_offsets
            .get(index)
            .ok_or(IndexedTdfBlobReaderError::InvalidIndex(index))?;
        let blob = self.blob_reader.get(offset)?;
        Ok(blob)
    }
}

#[derive(Debug, thiserror::Error)]
enum TdfBlobReaderError {
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
    MiniTDFPathError(#[from] MiniTDFPathError),
    #[error("{0}")]
    FileError(#[from] BinaryError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum IndexedTdfBlobReaderError {
    #[error("{0}")]
    #[allow(private_interfaces)]
    TdfBlobReaderError(#[from] TdfBlobReaderError),
    #[error("Invalid index {0}")]
    InvalidIndex(usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TdfBlob {
    bytes: Vec<u8>,
}

impl TdfBlob {
    #[allow(private_interfaces)]
    pub(crate) fn new(bytes: Vec<u8>) -> Result<Self, TdfBlobError> {
        if bytes.len().is_multiple_of(BLOB_TYPE_SIZE) {
            Ok(Self { bytes })
        } else {
            Err(TdfBlobError(bytes.len()))
        }
    }

    pub(crate) fn get_all(&self) -> Vec<u32> {
        (0..self.len())
            .map(|index| self.get(index).expect(
                "When iterating over the length of a tdf blob, you cannot go out of bounds"
            ))
            .collect()
    }

    pub(crate) fn get(&self, index: usize) -> Option<u32> {
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

    fn concatenate_bytes(b1: u8, b2: u8, b3: u8, b4: u8) -> u32 {
        b1 as u32
            | ((b2 as u32) << 8)
            | ((b3 as u32) << 16)
            | ((b4 as u32) << 24)
    }

    pub(crate) fn len(&self) -> usize {
        self.bytes.len() / BLOB_TYPE_SIZE
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Length {0} is not a multiple of {BLOB_TYPE_SIZE}")]
struct TdfBlobError(usize);
