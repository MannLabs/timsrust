#[derive(thiserror::Error, Debug)]
pub enum FileFormatError {
    #[error("DirectoryDoesNotExist")]
    DirectoryDoesNotExist,
    #[error("NoParentWithBrukerExtension")]
    NoParentWithBrukerExtension,
    #[error("BinaryFilesAreMissing")]
    BinaryFilesAreMissing,
    #[error("MetadataFilesAreMissing")]
    MetadataFilesAreMissing,
}

/// An error that is produced by timsrust (uses [thiserror]).
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// An error to indicate a path is not a Bruker File Format.
    #[error("FileFormatError: {0}")]
    FileFormatError(#[from] FileFormatError),
    // #[error("SqlError: {0}")]
    // SqlError(#[from] SqlError),
    // #[error("BinError: {0}")]
    // BinError(#[from] TdfBlobError),
}

#[macro_export]
macro_rules! propagated_error_enum {
    ($name:ident, $($variant:ident),+) => {
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            $(
                #[error(transparent)]
                $variant(#[from] $variant),
            )+
        }
    };
}
