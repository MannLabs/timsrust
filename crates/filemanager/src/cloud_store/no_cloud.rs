use crate::{CloudError, CloudProvider};

impl CloudProvider {
    pub(crate) fn parse(url: impl AsRef<str>) -> Option<Self> {
        None
    }
}

pub(crate) struct CloudObject {}

impl CloudObject {
    pub(crate) fn new(_url: impl AsRef<str>) -> Result<Self, CloudError> {
        Err(CloudError::FeatureNotEnabled)
    }

    #[allow(clippy::len_without_is_empty)]
    pub(crate) fn len(&self) -> Result<usize, CloudError> {
        Err(CloudError::FeatureNotEnabled)
    }

    pub(crate) fn children(&self) -> Result<Vec<String>, CloudError> {
        Err(CloudError::FeatureNotEnabled)
    }

    pub(crate) fn range(
        &self,
        _range: impl std::ops::RangeBounds<usize>,
    ) -> Result<Vec<u8>, CloudError> {
        Err(CloudError::FeatureNotEnabled)
    }

    pub(crate) fn download_to(
        &self,
        _dest: &std::path::Path,
    ) -> Result<(), CloudError> {
        Err(CloudError::FeatureNotEnabled)
    }

    pub(crate) fn upload_bytes(
        &self,
        bytes: Vec<u8>,
    ) -> Result<(), CloudError> {
        Err(CloudError::FeatureNotEnabled)
    }

    pub(crate) fn upload_from(
        &self,
        src: impl AsRef<std::path::Path>,
    ) -> Result<(), CloudError> {
        Err(CloudError::FeatureNotEnabled)
    }
}
