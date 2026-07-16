use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use object_store::{ObjectStore, path::Path as ObjectPath};
use url::Url;

use crate::{
    CloudError, cloud_store::CloudProvider, uri::URI_SCHEME_SEPARATOR,
};

/// Process-wide cache of authenticated [`ObjectStore`] instances.
///
/// Building an `ObjectStore` involves env-var lookups and constructing an
/// HTTP client, both of which are needlessly repeated when many URIs under
/// the same bucket/container are accessed. The cache is keyed by the parts
/// of the URI that determine store identity (scheme, host, and — for
/// virtual-hosted Azure HTTPS URLs — the container path segment).
fn store_cache() -> &'static RwLock<HashMap<String, Arc<dyn ObjectStore>>> {
    static CACHE: OnceLock<RwLock<HashMap<String, Arc<dyn ObjectStore>>>> =
        OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Computes the cache key for a parsed URL + provider.
///
/// For Azure HTTPS URLs the container is encoded into the bound store, so
/// it must be part of the key; for every other case `scheme://host` is
/// sufficient.
fn store_cache_key(url: &Url, provider: CloudProvider) -> String {
    let scheme = url.scheme();
    let host = url.host_str().unwrap_or("");
    if provider == CloudProvider::Azure && scheme == "https" {
        let container =
            url.path_segments().and_then(|mut s| s.next()).unwrap_or("");
        format!("{scheme}://{host}/{container}")
    } else {
        format!("{scheme}://{host}")
    }
}

fn build_store(
    url: &Url,
    host: &str,
    provider: CloudProvider,
) -> Result<Arc<dyn ObjectStore>, CloudError> {
    match provider {
        CloudProvider::S3 => build_s3(url, host),
        CloudProvider::Azure => build_azure(url, host),
        CloudProvider::Gcs => build_gcs(url, host),
    }
}

fn cached_or_build_store(
    url: &Url,
    host: &str,
    provider: CloudProvider,
) -> Result<Arc<dyn ObjectStore>, CloudError> {
    let key = store_cache_key(url, provider);
    let cache = store_cache();
    if let Some(store) = cache
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
    {
        return Ok(store.clone());
    }
    let store = build_store(url, host, provider)?;
    let mut guard = cache
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Ok(guard.entry(key).or_insert(store).clone())
}

// --- credential detection ---

fn env_any(vars: &[&str]) -> bool {
    vars.iter()
        .any(|v| std::env::var(v).is_ok_and(|val| !val.is_empty()))
}

fn has_azure_credentials(url: &Url) -> bool {
    if url.query().is_some_and(|q| !q.is_empty()) {
        return true; // SAS token embedded in URL
    }
    env_any(&[
        "AZURE_STORAGE_SAS_TOKEN",
        "AZURE_STORAGE_ACCOUNT_KEY",
        "AZURE_STORAGE_CONNECTION_STRING",
        "AZURE_CLIENT_SECRET",
        "AZURE_FEDERATED_TOKEN_FILE",
    ])
}

fn has_s3_credentials() -> bool {
    env_any(&[
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "AWS_PROFILE",
        "AWS_WEB_IDENTITY_TOKEN_FILE",
    ])
}

fn has_gcs_credentials() -> bool {
    env_any(&[
        "GOOGLE_SERVICE_ACCOUNT_KEY",
        "GOOGLE_SERVICE_ACCOUNT",
        "GOOGLE_APPLICATION_CREDENTIALS",
        "SERVICE_ACCOUNT",
    ])
}

// --- store builders ---

/// Returns the subdomain (first dot-segment of the host) for HTTPS URLs.
/// Used to derive bucket/account names from virtual-hosted-style URLs.
fn https_subdomain(url: &Url) -> Option<&str> {
    if url.scheme() != "https" {
        return None;
    }
    url.host_str()?.split('.').next()
}

fn build_s3(url: &Url, host: &str) -> Result<Arc<dyn ObjectStore>, CloudError> {
    if !has_s3_credentials() {
        return Err(CloudError::Authentication(
            "no S3 credentials found; set AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY, \
             AWS_PROFILE, or AWS_WEB_IDENTITY_TOKEN_FILE"
                .into(),
        ));
    }
    let bucket = https_subdomain(url).unwrap_or(host);
    let store = object_store::aws::AmazonS3Builder::from_env()
        .with_bucket_name(bucket)
        .build()
        .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
    Ok(Arc::new(store))
}

fn build_azure(
    url: &Url,
    host: &str,
) -> Result<Arc<dyn ObjectStore>, CloudError> {
    if !has_azure_credentials(url) {
        return Err(CloudError::Authentication(
            "no Azure credentials found; use a SAS URL, or set AZURE_STORAGE_ACCOUNT_KEY, \
             AZURE_STORAGE_SAS_TOKEN, AZURE_STORAGE_CONNECTION_STRING, or AZURE_CLIENT_SECRET"
                .into(),
        ));
    }
    // For https://<account>.blob.core.windows.net/<container>/... the container is the first
    // path segment; for native schemes (az://, abfs://, ...) the host is the container.
    let (account, container) = if url.scheme() == "https" {
        let account = https_subdomain(url).unwrap_or("");
        let container =
            url.path_segments().and_then(|mut s| s.next()).unwrap_or("");
        (account, container)
    } else {
        ("", host)
    };
    let mut builder = object_store::azure::MicrosoftAzureBuilder::from_env()
        .with_container_name(container);
    if !account.is_empty() {
        builder = builder.with_account(account);
    }
    // Apply SAS token query parameters from blob SAS URLs directly.
    let sas_pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if !sas_pairs.is_empty() {
        builder = builder.with_sas_authorization(sas_pairs);
    }
    let store = builder
        .build()
        .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
    Ok(Arc::new(store))
}

fn build_gcs(
    url: &Url,
    host: &str,
) -> Result<Arc<dyn ObjectStore>, CloudError> {
    if !has_gcs_credentials() {
        return Err(CloudError::Authentication(
            "no GCS credentials found; set GOOGLE_APPLICATION_CREDENTIALS or \
             GOOGLE_SERVICE_ACCOUNT_KEY"
                .into(),
        ));
    }
    let bucket = https_subdomain(url).unwrap_or(host);
    let store = object_store::gcp::GoogleCloudStorageBuilder::from_env()
        .with_bucket_name(bucket)
        .build()
        .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
    Ok(Arc::new(store))
}

// --- public interface ---

impl CloudProvider {
    pub(crate) fn parse(url: impl AsRef<str>) -> Option<Self> {
        let url = url.as_ref();
        let scheme =
            url.find(URI_SCHEME_SEPARATOR).map_or("", |pos| &url[..pos]);
        match scheme {
            "s3" | "s3a" => Some(Self::S3),
            "az" | "adl" | "azure" | "abfs" | "abfss" => Some(Self::Azure),
            "gs" => Some(Self::Gcs),
            "https" => Self::from_https_host(url),
            _ => None,
        }
    }

    fn from_https_host(url: impl AsRef<str>) -> Option<Self> {
        let url = Url::parse(url.as_ref()).ok()?;
        let host = url.host_str().unwrap_or("");
        if host.ends_with(".blob.core.windows.net") {
            Some(Self::Azure)
        } else if host.ends_with(".s3.amazonaws.com") {
            Some(Self::S3)
        } else if host.ends_with(".storage.googleapis.com") {
            Some(Self::Gcs)
        } else {
            None
        }
    }

    pub(crate) fn authenticate_store(
        url: impl AsRef<str>,
    ) -> Result<(Arc<dyn ObjectStore>, ObjectPath, String), CloudError> {
        let raw = url.as_ref();
        let url =
            Url::parse(raw).map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
        let host = url.host_str().unwrap_or("");
        let provider = CloudProvider::parse(&url)
            .ok_or_else(|| CloudError::NotACloudUri(url.to_string()))?;
        let store = cached_or_build_store(&url, host, provider)?;
        // For HTTPS Azure URLs the first path segment is the container, already baked into the
        // ObjectStore. Strip it so the resulting path is relative to the container.
        let path_str = url.path().strip_prefix('/').unwrap_or("");
        let path_str = if url.scheme() == "https"
            && matches!(provider, CloudProvider::Azure)
        {
            path_str.split_once('/').map(|(_, rest)| rest).unwrap_or("")
        } else {
            path_str
        };
        let scheme = url.scheme().to_string();
        let host = url.host_str().unwrap_or("").to_string();
        let base = format!("{}://{}", scheme, host);
        Ok((store, ObjectPath::from(path_str), base))
    }
}
