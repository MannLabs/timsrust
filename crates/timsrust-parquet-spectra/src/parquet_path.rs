use timsrust_core::io::Uri;
use timsrust_core::utils::custom_error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParquetSpectrumPath {
    uri: Uri,
    fragment_path: String,
    precursor_path: String,
}

impl ParquetSpectrumPath {
    pub fn new(
        path: impl AsRef<str>,
    ) -> Result<Self, ParquetSpectrumPathError> {
        let uri = Uri::from(path.as_ref());
        let fragment_uri = uri.join("fragments.parquet");
        let precursor_uri = uri.join("precursors.parquet");
        if fragment_uri.probe_is_file() && precursor_uri.probe_is_file() {
            return Ok(Self {
                uri,
                fragment_path: fragment_uri.as_ref().to_string(),
                precursor_path: precursor_uri.as_ref().to_string(),
            });
        }
        match uri.parent() {
            Some(parent) => Self::new(parent.as_ref()).map_err(|_| {
                ParquetSpectrumPathError(format!("Unknown path: {}", uri))
            }),
            None => {
                Err(ParquetSpectrumPathError(format!("Unknown path: {}", uri)))
            },
        }
    }

    pub fn fragment_path(&self) -> &String {
        &self.fragment_path
    }

    pub fn precursor_path(&self) -> &String {
        &self.precursor_path
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }
}

impl AsRef<str> for ParquetSpectrumPath {
    fn as_ref(&self) -> &str {
        self.uri.as_ref()
    }
}

pub trait ParquetSpectrumPathLike: AsRef<str> {
    fn to_timstof_path(
        &self,
    ) -> Result<ParquetSpectrumPath, ParquetSpectrumPathError>;
}

impl<T: AsRef<str>> ParquetSpectrumPathLike for T {
    fn to_timstof_path(
        &self,
    ) -> Result<ParquetSpectrumPath, ParquetSpectrumPathError> {
        ParquetSpectrumPath::new(self)
    }
}

custom_error!(pub ParquetSpectrumPathError);
