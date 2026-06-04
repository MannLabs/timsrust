use timsrust_core::io::Uri;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TDFPath {
    uri: Uri,
    tdf: Uri,
    tdf_bin: Uri,
}

impl TDFPath {
    pub fn new(path: impl AsRef<str>) -> Result<Self, TDFPathError> {
        let uri = Uri::from(path.as_ref());
        let tdf = uri.join("analysis.tdf");
        let tdf_bin = uri.join("analysis.tdf_bin");
        if tdf.probe_is_file() && tdf_bin.probe_is_file() {
            return Ok(Self { uri, tdf, tdf_bin });
        }
        match uri.parent() {
            Some(parent) => Self::new(parent.as_ref())
                .map_err(|_| TDFPathError::UnknownType(uri.to_string())),
            None => Err(TDFPathError::UnknownType(uri.to_string())),
        }
    }

    pub fn tdf(&self) -> &Uri {
        &self.tdf
    }

    pub fn tdf_bin(&self) -> &Uri {
        &self.tdf_bin
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }
}

impl AsRef<str> for TDFPath {
    fn as_ref(&self) -> &str {
        self.uri.as_ref()
    }
}

pub trait TDFPathLike: AsRef<str> {
    fn to_timstof_path(&self) -> Result<TDFPath, TDFPathError>;
}

impl<T: AsRef<str>> TDFPathLike for T {
    fn to_timstof_path(&self) -> Result<TDFPath, TDFPathError> {
        TDFPath::new(self)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TDFPathError {
    #[error("No valid type found for {0}")]
    UnknownType(String),
}
