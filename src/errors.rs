use crate::file_readers;

/// An error that is produced by timsrust (uses [thiserror]).
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("FileReaderError: {0}")]
    FileReaderError(#[from] file_readers::FileReaderError),
}
