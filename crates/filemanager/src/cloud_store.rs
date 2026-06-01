#[cfg(feature = "cloud")]
mod cloud;
#[cfg(not(feature = "cloud"))]
mod no_cloud;

#[cfg(feature = "cloud")]
pub(crate) use cloud::CloudObject;
#[cfg(not(feature = "cloud"))]
pub use no_cloud::CloudObject;

/// Errors that can occur when working with cloud stores.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CloudError {
    #[error(transparent)]
    ThirdParty(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("{0}")]
    Authentication(String),
    #[error("unsupported cloud URL: {0}")]
    NotACloudUri(String),
    #[error("cloud functionality requires the 'cloud' feature")]
    FeatureNotEnabled,
}

/// Maps a URL scheme to a cloud storage provider. This is the single source of
/// truth — `CLOUD_SCHEMES` is kept in sync with the variants below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloudProvider {
    S3,
    Azure,
    Gcs,
}
