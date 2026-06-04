use std::fmt;
use std::path::{Path, PathBuf};

use crate::cloud_store::CloudObject;
use crate::cloud_store::CloudProvider;

pub(crate) const URI_SCHEME_SEPARATOR: &str = "://";

/// Errors that can occur when working with [`Uri`](crate::Uri) values.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum UriError {
    /// The URI string is invalid or cannot be parsed.
    #[error("invalid URI: {0}")]
    InvalidUri(Uri),
    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A cloud storage error occurred.
    #[error("cloud error: {0}")]
    Cloud(#[from] crate::CloudError),
    /// The URI does not point to a directory.
    #[error("not a directory: {0}")]
    NotADirectory(String),
}

/// A URI referencing a local or cloud resource.
///
/// Local paths (with or without `file://` prefix) and cloud URIs
/// (`s3://`, `az://`, `gs://`, etc.) are all accepted.
///
/// # Examples
///
/// ```
/// use filemanager::Uri;
///
/// let uri = Uri::from("/tmp/data.bin");
/// assert!(uri.is_local());
/// assert_eq!(uri.scheme(), "");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Uri {
    raw: String,
}

impl Uri {
    /// Returns the scheme of the URI (`"s3"`, `"az"`, `"gs"`, `"file"`, or `""`).
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// assert_eq!(Uri::from("s3://bucket/key").scheme(), "s3");
    /// assert_eq!(Uri::from("/tmp/file.txt").scheme(), "");
    /// assert_eq!(Uri::from("file:///tmp/file.txt").scheme(), "file");
    /// ```
    pub fn scheme(&self) -> &str {
        self.raw
            .find(URI_SCHEME_SEPARATOR)
            .map_or("", |pos| &self.raw[..pos])
    }

    /// Returns the scheme of the URI (`"s3"`, `"az"`, `"gs"`, `"file"`, or `""`).
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// assert_eq!(Uri::from("s3://bucket/key").cloud_scheme().unwrap(), "s3");
    /// assert!(Uri::from("/tmp/file.txt").cloud_scheme().is_none());
    /// assert!(Uri::from("file:///tmp/file.txt").cloud_scheme().is_none());
    /// ```
    pub fn cloud_scheme(&self) -> Option<&str> {
        let scheme = self.scheme();
        if CloudProvider::parse(&self.raw).is_some() {
            Some(scheme)
        } else {
            None
        }
    }

    /// Returns the path component of this URI, stripping the scheme and
    /// authority (bucket / host).
    ///
    /// For cloud URIs (`s3://bucket/a/b`) this returns `"bucket"`.
    /// For local URIs (`/tmp/a/b` or `file:///tmp/a/b`) this returns `"tmp/a/b"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// assert_eq!(Uri::from("s3://my-bucket/data/file.parquet").bucket().unwrap(), "my-bucket");
    /// assert!(Uri::from("/tmp/data/file.txt").bucket().is_none());
    /// assert!(Uri::from("file:///tmp/data/file.txt").bucket().is_none());
    /// ```
    pub fn bucket(&self) -> Option<&str> {
        let scheme = self.cloud_scheme()?;
        let start = scheme.len() + URI_SCHEME_SEPARATOR.len();
        if let Some(end) = self.raw[start..].find('/') {
            self.raw.get(start..start + end)
        } else {
            // URI has authority but no path (e.g. "s3://bucket").
            self.raw.get(start..)
        }
    }

    /// Returns the path component of this URI, stripping the scheme and
    /// authority (bucket / host).
    ///
    /// For cloud URIs (`s3://bucket/a/b`) this returns `"a/b"`.
    /// For local URIs (`/tmp/a/b` or `file:///tmp/a/b`) this returns `"tmp/a/b"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// assert_eq!(Uri::from("s3://my-bucket/data/file.parquet").key().unwrap(), "data/file.parquet");
    /// assert!(Uri::from("/tmp/data/file.txt").key().is_none());
    /// assert!(Uri::from("file:///tmp/data/file.txt").key().is_none());
    /// ```
    pub fn key(&self) -> Option<&str> {
        let bucket = self.bucket()?;
        let scheme = self.scheme();
        let start = scheme.len()
            + URI_SCHEME_SEPARATOR.len()
            + bucket.len()
            + "/".len();
        self.raw.get(start..)
    }

    /// Returns `true` if this URI refers to a local file or path.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// assert!(Uri::from("/tmp/file.txt").is_local());
    /// assert!(Uri::from("file:///tmp/file.txt").is_local());
    /// assert!(!Uri::from("s3://bucket/key").is_local());
    /// ```
    pub fn is_local(&self) -> bool {
        matches!(self.scheme(), "file" | "")
    }

    /// Returns `true` if this URI refers to a cloud object store resource.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// assert!(Uri::from("s3://bucket/key").is_cloud());
    /// assert!(!Uri::from("/tmp/file.txt").is_cloud());
    /// ```
    pub fn is_cloud(&self) -> bool {
        self.cloud_scheme().is_some()
    }

    /// Returns `true` if this URI refers to a cloud object store resource.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// assert!(Uri::from("s3://bucket/key").is_valid());
    /// assert!(Uri::from("/tmp/file.txt").is_valid());
    /// assert!(!Uri::from("gibberish://bucket/key").is_valid());
    /// ```
    pub fn is_valid(&self) -> bool {
        self.is_local() || self.is_cloud()
    }

    /// Returns the parent URI, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// let uri = Uri::from("/tmp/subdir/file.txt");
    /// let parent = uri.parent().unwrap();
    /// assert_eq!(parent.as_ref(), "/tmp/subdir");
    /// ```
    pub fn parent(&self) -> Option<Uri> {
        if self.is_local() {
            self.as_path()?.parent().map(Uri::from)
        } else {
            // Cloud: strip any trailing slash, then take everything before the last '/'.
            // Guard against ascending past the bucket root (e.g. "s3://bucket").
            let raw = self.raw.trim_end_matches('/');
            let pos = raw.rfind('/')?;
            // The authority part ends with "://host", so if the prefix ends with ":/"
            // we have consumed all meaningful path segments.
            if raw[..pos].ends_with(":/") {
                return None;
            }
            Some(Uri {
                raw: raw[..pos].to_string(),
            })
        }
    }

    /// Lists child URIs (files and directories) under this URI.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// std::fs::write(dir.path().join("a.txt"), b"a").unwrap();
    /// std::fs::write(dir.path().join("b.txt"), b"b").unwrap();
    ///
    /// let uri = Uri::from(dir.path());
    /// let children = uri.list_children().unwrap();
    /// assert_eq!(children.len(), 2);
    /// ```
    pub fn list_children(&self) -> Result<Vec<Uri>, UriError> {
        if self.is_local() {
            let path = self
                .as_path()
                .ok_or_else(|| UriError::InvalidUri(self.clone()))?;
            let mut children = Vec::new();
            for entry in std::fs::read_dir(&path)? {
                let entry = entry?;
                children.push(Uri::from(entry.path()));
            }
            Ok(children)
        } else if self.is_cloud() {
            let cloud_object = CloudObject::new(self.as_ref())?;
            let children = cloud_object.children()?;
            Ok(children.into_iter().map(Uri::from).collect())
        } else {
            Err(UriError::NotADirectory(self.raw.clone()))
        }
    }

    /// Returns `true` if the resource exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// // A path that doesn't exist
    /// let uri = Uri::from("/nonexistent/path/xyz");
    /// assert!(!uri.exists().unwrap());
    /// ```
    pub fn exists(&self) -> Result<bool, UriError> {
        match self.as_path() {
            Some(path) => Ok(path.exists()),
            None => {
                if self.is_cloud() {
                    let _cloud_object = CloudObject::new(self.as_ref())?;
                    Ok(true)
                } else {
                    Err(UriError::InvalidUri(self.clone()))
                }
            },
        }
    }

    /// Returns `true` if the URI points to a regular file.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.txt");
    /// std::fs::write(&path, b"hello").unwrap();
    /// let uri = Uri::from(path);
    /// assert!(uri.is_file().unwrap());
    /// ```
    pub fn is_file(&self) -> Result<bool, UriError> {
        match self.as_path() {
            Some(path) => {
                if let Ok(path) = std::fs::metadata(&path) {
                    Ok(path.is_file())
                } else {
                    Ok(false)
                }
            },
            None => {
                if self.is_cloud() {
                    let cloud_object = CloudObject::new(self.as_ref())?;
                    let has_len = cloud_object.len().is_ok();
                    Ok(has_len)
                } else {
                    Err(UriError::InvalidUri(self.clone()))
                }
            },
        }
    }

    /// Returns `true` if the URI points to a directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let uri = Uri::from(dir.path());
    /// assert!(uri.is_folder().unwrap());
    /// ```
    pub fn is_folder(&self) -> Result<bool, UriError> {
        if self.is_local() {
            let path = self
                .as_path()
                .ok_or_else(|| UriError::InvalidUri(self.clone()))?;
            match std::fs::metadata(&path) {
                Ok(m) => Ok(m.is_dir()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(UriError::Io(e)),
            }
        } else if self.is_cloud() {
            use crate::cloud_store::CloudObject;

            // Explicit folder prefix — no need to probe the store.
            if self.raw.ends_with('/') {
                return Ok(true);
            }

            let cloud_object =
                CloudObject::new(self.as_ref()).map_err(UriError::Cloud)?;
            // If the path resolves to a concrete object it is a file, not a folder.
            if cloud_object.len().is_ok() {
                return Ok(false);
            }

            // No concrete object — check whether any objects exist under this
            // path as a prefix (handles the forgotten-trailing-slash case).
            let children = cloud_object.children().map_err(UriError::Cloud)?;
            Ok(!children.is_empty())
        } else {
            Err(UriError::InvalidUri(self.clone()))
        }
    }

    /// Appends a path segment to this URI.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    ///
    /// let uri = Uri::from("/tmp");
    /// let child = uri.join("data.bin");
    /// assert_eq!(child.as_ref(), "/tmp/data.bin");
    /// ```
    pub fn join(&self, segment: &str) -> Uri {
        if let Some(base) = self.as_path() {
            // Use std::path for local URIs so OS path rules are applied correctly.
            Uri::from(base.join(segment))
        } else {
            // Cloud URIs: simple string concatenation, always treating `segment`
            // as a relative component (strip any leading slash to avoid an
            // inadvertent double-slash or path reset).
            let raw = self.raw.trim_end_matches('/');
            let segment = segment.trim_start_matches('/');
            Uri {
                raw: format!("{}/{}", raw, segment),
            }
        }
    }

    /// Returns a `PathBuf` if this URI represents a local path.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::Uri;
    /// use std::path::PathBuf;
    ///
    /// assert_eq!(Uri::from("/tmp/file.txt").as_path(), Some(PathBuf::from("/tmp/file.txt")));
    /// assert_eq!(Uri::from("s3://bucket/key").as_path(), None);
    /// ```
    pub fn as_path(&self) -> Option<PathBuf> {
        // let local_uri = self.force_cache().ok()?;
        let local_uri = self;
        match local_uri.scheme() {
            "" => Some(PathBuf::from(&self.raw)),
            "file" => {
                let path_str =
                    &self.raw[("file".len() + URI_SCHEME_SEPARATOR.len())..];
                Some(PathBuf::from(path_str))
            },
            _ => None,
        }
    }

    /// Cache-aware existence check.
    ///
    /// For cloud URIs, consults the local cache first to avoid a network
    /// HEAD request when the file is already present on disk.
    pub fn probe_is_file(&self) -> bool {
        if let Some(local) = self.cached_local() {
            return local.is_file().unwrap_or(false);
        }
        self.is_file().unwrap_or(false)
    }
}

impl From<&str> for Uri {
    fn from(s: &str) -> Self {
        Uri { raw: s.to_string() }
    }
}

impl From<String> for Uri {
    fn from(s: String) -> Self {
        Uri { raw: s }
    }
}

impl From<&String> for Uri {
    fn from(s: &String) -> Self {
        Uri { raw: s.clone() }
    }
}

impl From<&Path> for Uri {
    fn from(p: &Path) -> Self {
        Uri {
            raw: p.to_string_lossy().into_owned(),
        }
    }
}

impl From<&PathBuf> for Uri {
    fn from(p: &PathBuf) -> Self {
        Uri {
            raw: p.to_string_lossy().into_owned(),
        }
    }
}

impl From<PathBuf> for Uri {
    fn from(p: PathBuf) -> Self {
        Uri {
            raw: p.to_string_lossy().into_owned(),
        }
    }
}

impl AsRef<str> for Uri {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}
