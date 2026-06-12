use timsrust_core::io::Uri;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TSFPath {
    uri: Uri,
    tsf: Uri,
    tsf_bin: Uri,
}

impl TSFPath {
    pub fn new(path: impl AsRef<str>) -> Result<Self, TSFPathError> {
        let uri = Uri::from(path.as_ref());
        let tsf = uri.join("analysis.tsf");
        let tsf_bin = uri.join("analysis.tsf_bin");
        if tsf.probe_is_file() && tsf_bin.probe_is_file() {
            return Ok(Self { uri, tsf, tsf_bin });
        }
        match uri.parent() {
            Some(parent) => Self::new(parent.as_ref())
                .map_err(|_| TSFPathError::UnknownType(uri.to_string())),
            None => Err(TSFPathError::UnknownType(uri.to_string())),
        }
    }

    pub fn tsf(&self) -> &Uri {
        &self.tsf
    }

    pub fn tsf_bin(&self) -> &Uri {
        &self.tsf_bin
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }
}

impl AsRef<str> for TSFPath {
    fn as_ref(&self) -> &str {
        self.uri.as_ref()
    }
}

pub trait TSFPathLike: AsRef<str> {
    fn to_timstof_path(&self) -> Result<TSFPath, TSFPathError>;
}

impl<T: AsRef<str>> TSFPathLike for T {
    fn to_timstof_path(&self) -> Result<TSFPath, TSFPathError> {
        TSFPath::new(self)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TSFPathError {
    #[error("No valid type found for {0}")]
    UnknownType(String),
}
