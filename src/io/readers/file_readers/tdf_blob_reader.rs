mod tdf_blobs;

use memmap2::Mmap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
pub use tdf_blobs::*;
use zstd::decode_all;

const U32_SIZE: usize = std::mem::size_of::<u32>();
const HEADER_SIZE: usize = 2;

#[derive(Debug)]
pub struct TdfBlobReader {
    path: PathBuf,
    mmap: Mmap,
    global_file_offset: usize,
}

impl TdfBlobReader {
    pub fn new(file_name: impl AsRef<Path>) -> Result<Self, TdfBlobError> {
        let path: PathBuf = file_name.as_ref().to_path_buf();
        let file: File = File::open(&path)?;
        let mmap: Mmap = unsafe { Mmap::map(&file)? };
        Ok(Self {
            path,
            mmap,
            global_file_offset: 0,
        })
    }

    pub fn get_blob(&self, offset: usize) -> Result<TdfBlob, TdfBlobError> {
        let offset: usize = self.get_offset(offset)?;
        let byte_count: usize = self.get_byte_count(offset)?;
        let compressed_bytes: &[u8] =
            self.get_compressed_bytes(offset, byte_count);
        match decode_all(compressed_bytes) {
            Ok(bytes) => Ok(TdfBlob::new(bytes)),
            Err(_) => Err(TdfBlobError::Decompression(self.path.clone())),
        }
    }

    fn get_offset(&self, offset: usize) -> Result<usize, TdfBlobError> {
        let offset = self.global_file_offset + offset;
        self.check_valid_offset(offset)
    }

    fn check_valid_offset(&self, offset: usize) -> Result<usize, TdfBlobError> {
        if (offset + U32_SIZE) >= self.mmap.len() {
            return Err(TdfBlobError::Offset(offset, self.path.clone()));
        }
        Ok(offset)
    }

    fn get_byte_count(&self, offset: usize) -> Result<usize, TdfBlobError> {
        let raw_byte_count: &[u8] =
            &self.mmap[offset as usize..(offset + U32_SIZE) as usize];
        let byte_count =
            u32::from_le_bytes(raw_byte_count.try_into().unwrap()) as usize;
        self.check_valid_byte_count(byte_count, offset)
    }

    fn check_valid_byte_count(
        &self,
        byte_count: usize,
        offset: usize,
    ) -> Result<usize, TdfBlobError> {
        if (byte_count < (HEADER_SIZE * U32_SIZE))
            || ((offset + byte_count) > self.len())
        {
            return Err(TdfBlobError::ByteCount(
                byte_count,
                offset,
                self.path.clone(),
            ));
        }
        Ok(byte_count)
    }

    fn get_compressed_bytes(&self, offset: usize, byte_count: usize) -> &[u8] {
        &self.mmap[(offset + HEADER_SIZE * U32_SIZE)..offset + byte_count]
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
    ) -> Result<Self, TdfBlobError> {
        Ok(Self {
            binary_offsets,
            blob_reader: TdfBlobReader::new(file_name)?,
        })
    }

    pub fn get_blob(&self, index: usize) -> Result<TdfBlob, TdfBlobError> {
        self.check_valid_index(index)?;
        let offset = self.binary_offsets[index];
        self.blob_reader.get_blob(offset)
    }

    fn check_valid_index(&self, index: usize) -> Result<usize, TdfBlobError> {
        if index >= self.len() {
            return Err(TdfBlobError::Index(
                index,
                self.blob_reader.path.clone(),
            ));
        }
        Ok(index)
    }

    pub fn len(&self) -> usize {
        self.binary_offsets.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TdfBlobError {
    #[error("Cannot read or mmap file {0}")]
    IO(#[from] io::Error),
    #[error("Index {0} is invalid for file {1}")]
    Index(usize, PathBuf),
    #[error("Offset {0} is invalid for file {1}")]
    Offset(usize, PathBuf),
    #[error("Byte count {0} from offset {1} is invalid for file {2}")]
    ByteCount(usize, usize, PathBuf),
    #[error("Zstd decompression failed for file {0}")]
    Decompression(PathBuf),
}
