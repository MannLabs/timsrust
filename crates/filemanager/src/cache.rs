use std::sync::{LazyLock, Mutex, OnceLock};

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
    pub fn cache(&self, uri: impl Into<Uri>) -> Result<Uri, CacheError> {
        match &self.dir {
            Some(dir) => {
                let uri = uri.into();
                if uri.is_local() {
                    return Ok(uri);
                }
                let key = uri.key().ok_or(UriError::InvalidUri(uri.clone()))?;
                let cache_path = dir.join(key);
                if cache_path.exists() {
                    Ok(Uri::from(cache_path))
                } else {
                    let tmp_path = dir.join(format!("{}.tmp", key));
                    let obj = CloudObject::new(uri)?;
                    obj.download_to(&tmp_path)?;
                    std::fs::rename(&tmp_path, &cache_path)?;
                    Ok(Uri::from(cache_path))
                }
            },
            None => Err(CacheError::NoCacheDirectory),
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
}
