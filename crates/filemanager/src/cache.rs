use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use tempfile::TempDir;

use crate::{cloud_store::CloudObject, CloudError, Uri, UriError};

static TEMP_DIR: OnceLock<TempDir> = OnceLock::new();

fn temp_dir() -> &'static std::path::Path {
    TEMP_DIR
        .get_or_init(|| TempDir::new().expect("temp dir"))
        .path()
}

static GLOBAL_CACHE: LazyLock<Mutex<FileCache>> = LazyLock::new(|| {
    let cache = std::env::var("FILEMANAGER_CACHE_DIR")
        .ok()
        .and_then(|dir| FileCache::persistent(dir).ok())
        .unwrap_or_default();
    Mutex::new(cache)
});

static IN_FLIGHT: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn in_flight_lock(path: &Path) -> Arc<Mutex<()>> {
    let mut map = IN_FLIGHT.lock().expect("in_flight map mutex poisoned");
    map.entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

pub fn set_global_cache(file_cache: FileCache) {
    *GLOBAL_CACHE.lock().unwrap() = file_cache;
}

pub fn global_cache() -> FileCache {
    GLOBAL_CACHE.lock().unwrap().clone()
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Cloud(#[from] CloudError),
    #[error("{0}")]
    Uri(#[from] UriError),
    #[error("No cache has been set")]
    NoCacheDirectory,
}

#[derive(Debug, Clone, Default)]
pub struct FileCache {
    dir: Option<std::path::PathBuf>,
}

impl FileCache {
    pub fn persistent(
        dir: impl Into<std::path::PathBuf>,
    ) -> Result<Self, CacheError> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(FileCache { dir: Some(dir) })
    }

    pub fn temp() -> Self {
        FileCache {
            dir: Some(temp_dir().to_path_buf()),
        }
    }

    pub fn none() -> Self {
        FileCache { dir: None }
    }

    /// Downloads (or passes through) `uri` into the cache directory.
    ///
    /// Local URIs are returned unchanged. Cloud URIs are downloaded atomically
    /// into the cache directory and the resulting local URI is returned.
    /// If the cache file already exists it is reused without re-downloading.
    /// Concurrent calls for the same URI within a process are coalesced:
    /// only one download runs while the others wait and reuse the result.
    pub fn cache(&self, uri: impl Into<Uri>) -> Result<Uri, CacheError> {
        let dir = self.dir.as_ref().ok_or(CacheError::NoCacheDirectory)?;
        let uri = uri.into();
        if uri.is_local() {
            return Ok(uri);
        }
        let key = uri.key().ok_or(UriError::InvalidUri(uri.clone()))?;
        let cache_path = dir.join(key);
        if cache_path.exists() {
            return Ok(Uri::from(cache_path));
        }
        let lock = in_flight_lock(&cache_path);
        let _guard = lock.lock().expect("in_flight per-key mutex poisoned");
        if cache_path.exists() {
            return Ok(Uri::from(cache_path));
        }
        let tmp_path = dir.join(format!("{}.tmp", key));
        let obj = CloudObject::new(uri)?;
        obj.download_to(&tmp_path)?;
        std::fs::rename(&tmp_path, &cache_path)?;
        Ok(Uri::from(cache_path))
    }

    /// Returns the local cache URI for `uri` iff a cache file already exists.
    ///
    /// Never downloads, never issues network requests, never errors.
    /// Local URIs pass through unchanged. For cloud URIs, returns
    /// `Some(local_uri)` only when the corresponding cache file is already
    /// present on disk; otherwise returns `None`.
    ///
    /// This is the cache-aware probe used by callers (such as path
    /// resolvers) that would otherwise issue cloud HEAD requests to check
    /// for file existence.
    pub fn cached_local(&self, uri: impl Into<Uri>) -> Option<Uri> {
        let uri = uri.into();
        if uri.is_local() {
            return Some(uri);
        }
        let dir = self.dir.as_ref()?;
        let key = uri.key()?;
        let cache_path = dir.join(key);
        if cache_path.is_file() {
            Some(Uri::from(cache_path))
        } else {
            None
        }
    }

    pub fn invalidate(&self, uri: impl Into<Uri>) -> Result<(), CacheError> {
        match &self.dir {
            Some(dir) => {
                let uri = uri.into();
                let cache_path = match uri.as_path() {
                    Some(p) => {
                        if p.starts_with(dir) {
                            p.to_path_buf()
                        } else {
                            return Ok(()); // Not in cache, nothing to invalidate
                        }
                    },
                    None => {
                        let key = uri
                            .key()
                            .ok_or(UriError::InvalidUri(uri.clone()))?;
                        dir.join(key)
                    },
                };
                Ok(std::fs::remove_file(cache_path)?)
            },
            None => Ok(()), // No cache, nothing to invalidate
        }
    }
}

impl Uri {
    pub fn try_cache(&self) -> Result<Uri, CacheError> {
        let file_cache = crate::global_cache();
        file_cache.cache(self.clone())
    }

    pub fn force_cache(&self) -> Result<Uri, CacheError> {
        match self.try_cache() {
            Ok(uri) => Ok(uri),
            Err(_) => {
                let cache = FileCache::temp();
                cache.cache(self.clone())
            },
        }
    }

    pub fn soft_cache(&self) -> Uri {
        self.try_cache().unwrap_or_else(|_| self.clone())
    }

    /// Returns the local cache URI iff this URI is already cached on disk.
    ///
    /// Never downloads, never issues network requests. Local URIs pass
    /// through unchanged. For cloud URIs, returns `Some(local_uri)` only
    /// when the cache file already exists; otherwise returns `None`.
    pub fn cached_local(&self) -> Option<Uri> {
        crate::global_cache().cached_local(self.clone())
    }
}
