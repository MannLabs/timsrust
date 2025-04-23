mod tdf_blobs;

use lzf::decompress as lzf_decompress;
use memmap2::Mmap;
use std::fs::File;
use std::io;
pub use tdf_blobs::*;
use zstd::decode_all;

use crate::readers::{TimsTofFileType, TimsTofPathError, TimsTofPathLike};

const U32_SIZE: usize = std::mem::size_of::<u32>();

const HEADER_SIZE: usize = 2;

#[derive(Debug)]
pub struct TdfBlobReader {
    bin_file_reader: TdfBinFileReader,
}

impl TdfBlobReader {
    pub fn new(path: impl TimsTofPathLike) -> Result<Self, TdfBlobReaderError> {
        let bin_file_reader = TdfBinFileReader::new(path)?;
        let reader = Self { bin_file_reader };
        Ok(reader)
    }

    /// Returns a TDF blob with uncompressed data
    ///
    pub fn get(
        &self,
        offset: usize,
        compression_type: u8,
        max_peaks_per_scan: usize,
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
        if data.len() == 0 {
            return Err(TdfBlobReaderError::EmptyData);
        }
        if compression_type == 1 {
            let bytes = self.decompress_v1(
                offset,
                byte_count,
                data,
                max_peaks_per_scan,
            )?;
            let blob = TdfBlob::new(bytes)?;
            Ok(blob)
        } else {
            let bytes = decode_all(data)
                .map_err(|_| TdfBlobReaderError::Decompression)?;
            let blob: TdfBlob = TdfBlob::new(bytes)?;
            Ok(blob)
        }
    }

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
        byte_count: usize,
        data: &[u8],
        max_peaks_per_scan: usize,
    ) -> Result<Vec<u8>, TdfBlobReaderError> {
        // bin_size = int.from_bytes(infile.read(4), "little")
        // bin_size == byte_count

        // scan_count = int.from_bytes(infile.read(4), "little")
        let scan_count = self
            .bin_file_reader
            .get_scan_count(offset)
            .ok_or(TdfBlobReaderError::NoScanCount)?;

        // TODO: frame_end - frame_start should be equal to bin_size/byte_count?
        // max_peak_count = min(
        //     max_peaks_per_scan,
        //     frame_end - frame_start
        // )
        let max_peak_count = std::cmp::min(max_peaks_per_scan, byte_count);

        // compression_offset = 8 + (scan_count + 1) * 4
        let compression_offset = (scan_count + 1) * U32_SIZE;

        // TODO: For some reason scan offsets were i32 not u32. Convert to u32 than to usize for easier indexing
        // scan_offsets = np.frombuffer(
        //     infile.read((scan_count + 1) * 4),
        //     dtype=np.int32
        // ) - compression_offset
        let mut scan_offsets = self
            .bin_file_reader
            .get_scan_offsets(offset, scan_count)
            .ok_or(TdfBlobReaderError::CorruptData)?;
        scan_offsets = scan_offsets.iter_mut().map(|x| *x - compression_offset).collect();

        // bin_size + scan_count + scan_offsets + compressed_data (which is max_peak_count * 4)
        let tdf_bytes_capacity = U32_SIZE + U32_SIZE + scan_offsets.len() * U32_SIZE + scan_count * max_peak_count;

        // this is basically the uncompressed frame
        // scan_count: 4 bytes
        // scan_indices: (scan_count) * 4 bytes
        // scan: remaining bytes
        let mut tdf_bytes = Vec::with_capacity(tdf_bytes_capacity);
        tdf_bytes.extend_from_slice(&byte_count.to_le_bytes());
    

        let mut scan_indexes: Vec<u8> = Vec::with_capacity(scan_count * U32_SIZE);
        let mut scans: Vec<u8> = Vec::with_capacity(byte_count);

        let mut scan_start: u32 = 0;
        // for scan_index in range(scan_count):
        for scan_index in 0..scan_count {
            //start = scan_offsets[scan_index]
            let start = scan_offsets[scan_index];

            //end = scan_offsets[scan_index + 1]
            let end = scan_offsets[scan_index + 1];

            //if start == end:
            //    continue
            if start == end {
                continue;
            }

            //decompressed_bytes = lzf.decompress(
            //    compressed_data[start: end],
            //    max_peak_count * 4 * 2
            //)
            let mut decompressed_bytes = match lzf_decompress(
                &data[start as usize..end as usize],
                max_peak_count * U32_SIZE * 2,
            ) {
                Ok(bytes) => bytes,
                Err(_) => return Err(TdfBlobReaderError::Decompression),
            };
            
            if decompressed_bytes.len() % U32_SIZE != 0 {
                return Err(TdfBlobReaderError::CorruptData);
            }

            scan_indexes.extend_from_slice(&scan_start.to_le_bytes());
            scan_start = decompressed_bytes.len() as u32;
            scans.append(&mut decompressed_bytes);
        }

        tdf_bytes.append(&mut scan_indexes);
        tdf_bytes.append(&mut scans);

        Ok(tdf_bytes)
    }
}

#[derive(Debug)]
struct TdfBinFileReader {
    mmap: Mmap,
    global_file_offset: usize,
}

impl TdfBinFileReader {
    // TODO parse compression1
    fn new(path: impl TimsTofPathLike) -> Result<Self, TdfBlobReaderError> {
        let path = path.to_timstof_path()?;
        let bin_path = match path.file_type() {
            #[cfg(feature = "tdf")]
            TimsTofFileType::TDF => path.tdf_bin()?,
            #[cfg(feature = "minitdf")]
            TimsTofFileType::MiniTDF => path.ms2_bin()?,
        };
        let file = File::open(bin_path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let reader = Self {
            mmap,
            global_file_offset: 0,
        };
        Ok(reader)
    }

    /// Get byte count, first 4 bytes of the blob
    /// 
    fn get_byte_count(&self, offset: usize) -> Option<usize> {
        let start = offset as usize;
        let end = start + U32_SIZE as usize;
        let raw_byte_count = self.mmap.get(start..end)?;
        let byte_count =
            u32::from_le_bytes(raw_byte_count.try_into().ok()?) as usize;
        Some(byte_count)
    }

    /// Get scan count, second 4 bytes of the blob
    /// 
    fn get_scan_count(&self, offset: usize) -> Option<usize> {
        let start = (offset + U32_SIZE) as usize;
        let end = start + U32_SIZE as usize;
        let raw_scan_count = self.mmap.get(start..end)?;
        let scan_count =
            u32::from_le_bytes(raw_scan_count.try_into().ok()?) as usize;
        Some(scan_count)
    }

    /// Get scan offsets, third 4 bytes of the blob
    /// 
    fn get_scan_offsets(&self, offset: usize, scan_count: usize) -> Option<Vec<usize>> {
        let start = (offset + U32_SIZE * 2) as usize;
        let end = start + U32_SIZE * (scan_count + 1) as usize;
        let raw_scan_offsets = self.mmap.get(start..end)?;
        if raw_scan_offsets.len() % U32_SIZE != 0 {
            return None;
        }
        let scan_offsets = raw_scan_offsets
            .chunks_exact(U32_SIZE)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()))
            .map(|x| x as usize)
            .collect::<Vec<usize>>();
        Some(scan_offsets)
    }

    fn get_data(&self, offset: usize, byte_count: usize) -> Option<&[u8]> {
        let start = offset + HEADER_SIZE * U32_SIZE;
        let end = offset + byte_count;
        self.mmap.get(start..end)
    }
}

#[cfg(feature = "minitdf")]
#[derive(Debug)]
pub struct IndexedTdfBlobReader {
    blob_reader: TdfBlobReader,
    binary_offsets: Vec<usize>,
}

#[cfg(feature = "minitdf")]
impl IndexedTdfBlobReader {
    pub fn new(
        path: impl TimsTofPathLike,
        binary_offsets: Vec<usize>,
    ) -> Result<Self, IndexedTdfBlobReaderError> {

        let blob_reader = TdfBlobReader::new(path)?;
        let reader = Self {
            binary_offsets,
            blob_reader,
        };
        Ok(reader)
    }

    pub fn get(
        &self,
        index: usize,
    ) -> Result<TdfBlob, IndexedTdfBlobReaderError> {
        let offset = *self
            .binary_offsets
            .get(index)
            .ok_or(IndexedTdfBlobReaderError::InvalidIndex(index))?;
        let blob = self.blob_reader.get(
            offset, 
            // TODO: Compression type 1 seems to be irrelevant for minitdf. Correct? 
            // Set compression to type 2 for latest compression and max peaks to 0 which is only relevant for type 1
            2, 
            0 
        )?;
        Ok(blob)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TdfBlobReaderError {
    #[error("{0}")]
    IO(#[from] io::Error),
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
    TimsTofPathError(#[from] TimsTofPathError),
    #[error("No binary file found")]
    NoBinary,
    #[error("No scan count found")]
    NoScanCount,
    #[error("No binary size found")]
    NoBinarySize,
    #[error("Scan offset error")]
    ScanOffsetError,
    #[error("No scan offsets found")]
    NoScanOffsets,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexedTdfBlobReaderError {
    #[error("{0}")]
    TdfBlobReaderError(#[from] TdfBlobReaderError),
    #[cfg(feature = "minitdf")]
    #[error("Invalid index {0}")]
    InvalidIndex(usize),
}
