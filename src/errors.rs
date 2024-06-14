use crate::{
    file_readers,
    // io::readers::common::{sql_reader::SqlError, tdf_blobs::TdfBlobError},
};

/// An error that is produced by timsrust (uses [thiserror]).
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// An error to indicate a path is not a Bruker File Format.
    #[error("FileFormatError: {0}")]
    FileFormatError(#[from] file_readers::FileFormatError),
    // #[error("SqlError: {0}")]
    // SqlError(#[from] SqlError),
    // #[error("BinError: {0}")]
    // BinError(#[from] TdfBlobError),
}
