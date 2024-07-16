mod tdf_blobs;

use memmap2::Mmap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
pub use tdf_blobs::*;
use zstd::decode_all;

const BLOB_TYPE_SIZE: usize = std::mem::size_of::<u32>();
const HEADER_SIZE: usize = 2;

#[derive(Debug)]
pub struct TdfBlobReader {
    path: PathBuf,
    mmap: Mmap,
    global_file_offset: usize,
}

impl TdfBlobReader {
    // TODO parse compression1
    pub fn new(
        file_name: impl AsRef<Path>,
    ) -> Result<Self, TdfBlobReaderError> {
        let path = file_name.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let reader = Self {
            path,
            mmap,
            global_file_offset: 0,
        };
        Ok(reader)
    }

    pub fn get_blob(
        &self,
        offset: usize,
    ) -> Result<TdfBlob, TdfBlobReaderError> {
        let offset = self.global_file_offset + offset;
        let byte_count = self.get_byte_count(offset)?;
        let compressed_bytes = self.get_compressed_bytes(offset, byte_count)?;
        let bytes = decode_all(compressed_bytes)?;
        let blob = TdfBlob::new(bytes)?;
        Ok(blob)
    }

    fn get_byte_count(
        &self,
        offset: usize,
    ) -> Result<usize, TdfBlobReaderError> {
        let start = offset as usize;
        let end = (offset + BLOB_TYPE_SIZE) as usize;
        let raw_byte_count = self.mmap.get(start..end).ok_or(
            TdfBlobReaderError::RangeOutOfBounds {
                start,
                end,
                length: self.mmap.len(),
            },
        )?;
        let byte_count =
            u32::from_le_bytes(raw_byte_count.try_into().unwrap()) as usize;
        Ok(byte_count)
    }

    fn get_compressed_bytes(
        &self,
        offset: usize,
        byte_count: usize,
    ) -> Result<&[u8], TdfBlobReaderError> {
        let start = offset + HEADER_SIZE * BLOB_TYPE_SIZE;
        let end = offset + byte_count;
        self.mmap
            .get(start..end)
            .ok_or(TdfBlobReaderError::RangeOutOfBounds {
                start,
                end,
                length: self.mmap.len(),
            })
    }

    pub fn len(&self) -> usize {
        self.mmap.len()
    }
}

#[derive(Debug)]
pub struct IndexedTdfBlobReader {
    blob_reader: TdfBlobReader,
    binary_offsets: Vec<usize>,
}

impl IndexedTdfBlobReader {
    pub fn new(
        file_name: impl AsRef<Path>,
        binary_offsets: Vec<usize>,
    ) -> Result<Self, TdfBlobReaderError> {
        let blob_reader = TdfBlobReader::new(file_name)?;
        let reader = Self {
            binary_offsets,
            blob_reader: blob_reader,
        };
        Ok(reader)
    }

    pub fn get_blob(
        &self,
        index: usize,
    ) -> Result<TdfBlob, TdfBlobReaderError> {
        let offset = *self.binary_offsets.get(index).ok_or(
            TdfBlobReaderError::IndexOutOfBounds {
                index,
                length: self.binary_offsets.len(),
            },
        )?;
        self.blob_reader.get_blob(offset)
    }

    pub fn len(&self) -> usize {
        self.binary_offsets.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TdfBlobReaderError {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("{0}")]
    TdfBlob(#[from] TdfBlobError),
    #[error("Index {index} out of bounds for length {length})")]
    IndexOutOfBounds { index: usize, length: usize },
    #[error("Range [{start}-{end}] out of bounds for length {length})")]
    RangeOutOfBounds {
        start: usize,
        end: usize,
        length: usize,
    },

    #[error("Index {0} is invalid for file {1}")]
    Index(usize, PathBuf),
    #[error("Offset {0} is invalid for file {1}")]
    Offset(usize, PathBuf),
    #[error("Byte count {0} from offset {1} is invalid for file {2}")]
    ByteCount(usize, usize, PathBuf),
}
