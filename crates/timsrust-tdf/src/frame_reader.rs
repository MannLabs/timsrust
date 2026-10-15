pub(crate) mod compression1;
pub(crate) mod compression2;
pub(crate) mod frame_info_reader;

use std::collections::{HashMap, HashSet};

use timsrust_core::{FrameIons, utils::reader::Reader};

pub(crate) use frame_info_reader::FrameInfoReader;

use crate::{
    Metadata, TDFPathError,
    frame_reader::{
        compression1::{
            TdfBlobReaderCompression1, TdfBlobReaderErrorCompression1,
        },
        compression2::{TdfBlobReader, TdfBlobReaderError},
    },
};

pub use frame_info_reader::FrameReaderErrorInternal;

use super::{
    MetadataReaderError, QuadrupoleSettingsReaderError, TDFPathLike,
    file_readers::sql_reader::SqlReaderError,
};

/// Adapts a raw blob reader (indexed by binary file offset) to be indexed by
/// frame id, using the offset table from [`FrameInfoReader`].
#[derive(Debug)]
struct TdfOffsetIonReader<B> {
    blob_reader: B,
    offsets: HashMap<usize, usize>,
    empty_frames: HashSet<usize>,
}

impl<B> TdfOffsetIonReader<B> {
    fn new(
        blob_reader: B,
        offsets: HashMap<usize, usize>,
        empty_frames: HashSet<usize>,
    ) -> Self {
        Self {
            blob_reader,
            offsets,
            empty_frames,
        }
    }
}

impl<B: Reader<FrameIons>> Reader<FrameIons> for TdfOffsetIonReader<B>
where
    FrameReaderError: From<B::Error>,
{
    type Error = FrameReaderError;

    fn get(&self, index: usize) -> Result<FrameIons, Self::Error> {
        let offset = self
            .offsets
            .get(&index)
            .copied()
            .ok_or(FrameReaderError::IndexOutOfBounds)?;
        if self.empty_frames.contains(&index) {
            return Ok(FrameIons::default());
        }
        self.blob_reader.get(offset).map_err(FrameReaderError::from)
    }
}

/// Unifies the two TDF compression schemes into a single [`Reader<FrameIons>`].
#[allow(private_interfaces)]
#[derive(Debug)]
pub enum TdfIonReader {
    Compression1(TdfOffsetIonReader<TdfBlobReaderCompression1>),
    Compression2(TdfOffsetIonReader<TdfBlobReader>),
}

impl Reader<FrameIons> for TdfIonReader {
    type Error = FrameReaderError;

    fn get(&self, index: usize) -> Result<FrameIons, FrameReaderError> {
        match self {
            Self::Compression1(r) => r.get(index),
            Self::Compression2(r) => r.get(index),
        }
    }
}

/// A concrete frame reader for Bruker TDF files.
///
/// Thin newtype around
/// [`timsrust_core::FrameReader<TdfIonReader, FrameInfoReader>`].  All
/// [`FrameReader`] methods (`get_frame`, `get_info`, `iter_indices`,
/// `parallel_filter`, …) are available via [`Deref`].
#[derive(Debug)]
pub struct TdfFrameReader(
    timsrust_core::FrameReader<TdfIonReader, FrameInfoReader>,
);

impl std::ops::Deref for TdfFrameReader {
    type Target = timsrust_core::FrameReader<TdfIonReader, FrameInfoReader>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TdfFrameReader {
    pub fn new(path: impl TDFPathLike) -> Result<Self, FrameReaderError> {
        let metadata = Metadata::new(&path)?;
        Self::without_metadata(
            &path,
            metadata.compression_type(),
            metadata.max_peaks_per_scan(),
        )
    }

    pub fn without_metadata(
        path: impl TDFPathLike,
        compression_type: u8,
        max_peaks_per_scan: usize,
    ) -> Result<Self, FrameReaderError> {
        let info_reader = FrameInfoReader::new(&path)?;
        let offsets = info_reader.offsets_map();
        let empty_frames = info_reader.empty_frames();
        let ion_reader = match compression_type {
            1 => {
                let mut blob =
                    TdfBlobReaderCompression1::new(path.to_timstof_path())?;
                blob.set_max_peaks_per_scan(max_peaks_per_scan);
                TdfIonReader::Compression1(TdfOffsetIonReader::new(
                    blob,
                    offsets,
                    empty_frames,
                ))
            },
            2 => {
                let blob = TdfBlobReader::new(path.to_timstof_path())?;
                TdfIonReader::Compression2(TdfOffsetIonReader::new(
                    blob,
                    offsets,
                    empty_frames,
                ))
            },
            _ => {
                return Err(FrameReaderError::CompressionTypeError(
                    compression_type,
                ));
            },
        };
        Ok(Self(timsrust_core::FrameReader::new(
            ion_reader,
            info_reader,
        )))
    }

    /// Consume `self` and return the underlying generic
    /// [`timsrust_core::FrameReader`].
    pub fn into_inner(
        self,
    ) -> timsrust_core::FrameReader<TdfIonReader, FrameInfoReader> {
        self.0
    }

    /// Return the acquisition type detected from the frame metadata.
    pub fn get_acquisition(&self) -> timsrust_core::AcquisitionType {
        self.info_reader().get_acquisition()
    }
}

#[allow(private_interfaces)]
#[derive(Debug, thiserror::Error)]
pub enum FrameReaderError {
    #[error("Timscompress error")]
    TimscompressError,
    #[error("{0}")]
    TdfBlobReaderError(#[from] TdfBlobReaderError),
    #[error("{0}")]
    TdfBlobReaderErrorCompression1(#[from] TdfBlobReaderErrorCompression1),
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    FileNotFound(String),
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("Corrupt Frame")]
    CorruptFrame,
    #[error("{0}")]
    QuadrupoleSettingsReaderError(#[from] QuadrupoleSettingsReaderError),
    #[error("Index out of bounds")]
    IndexOutOfBounds,
    #[error("Compression type {0} not understood")]
    CompressionTypeError(u8),
    #[error("Failed to read path: {0}")]
    PathError(#[from] TDFPathError),
    #[error("Got unexpected TdfBlob type")]
    UnexpectedTdfBlobError,
    #[error("{0}")]
    FrameInfoReaderError(#[from] frame_info_reader::FrameReaderErrorInternal),
    #[error("{0}")]
    CoreFrameReaderError(#[from] timsrust_core::FrameReaderError),
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        collections::{HashMap, HashSet},
    };

    use timsrust_core::{FrameIons, utils::reader::Reader};

    use super::{FrameReaderError, TdfOffsetIonReader};
    use crate::frame_reader::compression2::TdfBlobReaderError;

    struct FailingBlobReader(Cell<usize>);

    impl Reader<FrameIons> for FailingBlobReader {
        type Error = TdfBlobReaderError;

        fn get(&self, _offset: usize) -> Result<FrameIons, Self::Error> {
            self.0.set(self.0.get() + 1);
            Err(TdfBlobReaderError::CorruptFrame)
        }
    }

    #[test]
    fn zero_peak_frame_skips_the_binary_reader_without_masking_other_errors() {
        let reader = TdfOffsetIonReader::new(
            FailingBlobReader(Cell::new(0)),
            HashMap::from([(1, 0), (2, 8)]),
            HashSet::from([1]),
        );

        assert_eq!(reader.get(1).unwrap(), FrameIons::default());
        assert_eq!(reader.blob_reader.0.get(), 0);
        assert!(matches!(
            reader.get(2),
            Err(FrameReaderError::TdfBlobReaderError(
                TdfBlobReaderError::CorruptFrame
            ))
        ));
        assert_eq!(reader.blob_reader.0.get(), 1);
        assert!(matches!(
            reader.get(3),
            Err(FrameReaderError::IndexOutOfBounds)
        ));
    }
}
