use crate::file_readers;

/// An error that is produced by timsrust (uses [thiserror]).
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// An error to indicate a path is not a Bruker File Format.
    #[error("FileFormatError: {0}")]
    FileFormatError(#[from] file_readers::FileFormatError),
}
