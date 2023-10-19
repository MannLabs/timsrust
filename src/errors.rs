use crate::file_readers;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("FileReaderError: {0}")]
    FileReaderError(#[from] file_readers::FileReaderError),
}
