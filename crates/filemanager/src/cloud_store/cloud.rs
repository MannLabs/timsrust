mod authenticate;

use crate::{cloud_store::CloudProvider, runtime::block_on, CloudError};

use futures::StreamExt;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub(crate) struct CloudObject {
    store: Arc<dyn object_store::ObjectStore>,
    path: object_store::path::Path,
    base: String,
}

impl CloudObject {
    pub(crate) fn new(url: impl AsRef<str>) -> Result<Self, CloudError> {
        let (store, path, base) =
            CloudProvider::authenticate_store(url.as_ref())?;
        Ok(CloudObject { store, path, base })
    }

    #[allow(clippy::len_without_is_empty)]
    pub(crate) fn len(&self) -> Result<usize, CloudError> {
        Ok(block_on(self.store.head(&self.path))
            .map_err(|e| CloudError::ThirdParty(Box::new(e)))?
            .size)
    }

    pub(crate) fn children(&self) -> Result<Vec<String>, CloudError> {
        let mut children = vec![];
        let prefix = Some(self.path.clone());
        let list_result =
            block_on(self.store.list_with_delimiter(prefix.as_ref()))
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
        for obj in list_result.objects {
            children.push(format!("{}/{}", &self.base, obj.location));
        }
        for prefix in list_result.common_prefixes {
            children.push(format!("{}/{}/", &self.base, prefix));
        }
        Ok(children)
    }

    pub(crate) fn range(
        &self,
        range: impl std::ops::RangeBounds<usize>,
    ) -> Result<Vec<u8>, CloudError> {
        let range = crate::range(range, self.len()?);
        let data = block_on(self.store.get_range(&self.path, range))
            .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
        Ok(data.to_vec())
    }

    pub(crate) fn download_to(
        &self,
        dest: &std::path::Path,
    ) -> Result<(), CloudError> {
        let expected_size = match self.len() {
            Ok(n) => n,
            Err(_) => return Ok(()), // Object doesn't exist, skip download
        };
        block_on(async {
            let result = self
                .store
                .get(&self.path)
                .await
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;

            let mut stream = result.into_stream();
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
            }
            let mut file = tokio::fs::File::create(dest)
                .await
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;

            let mut bytes_written: usize = 0;
            while let Some(chunk) = stream.next().await {
                let bytes =
                    chunk.map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
                bytes_written += bytes.len();
                file.write_all(&bytes)
                    .await
                    .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
            }

            file.sync_data()
                .await
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;

            let actual_size = file
                .metadata()
                .await
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?
                .len() as usize;

            if actual_size != expected_size {
                return Err(CloudError::ThirdParty(Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!(
                            "truncated download: expected {} bytes, stream delivered {}, on-disk {}",
                            expected_size, bytes_written, actual_size
                        ),
                    ),
                )));
            }
            Ok(())
        })
    }

    pub(crate) fn upload_bytes(
        &self,
        bytes: Vec<u8>,
    ) -> Result<(), CloudError> {
        block_on(async move {
            self.store
                .put(&self.path, bytes.into())
                .await
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;
            Ok(())
        })
    }

    pub(crate) fn upload_from(
        &self,
        src: impl AsRef<std::path::Path>,
    ) -> Result<(), CloudError> {
        let src = src.as_ref();
        block_on(async {
            let mut file = tokio::fs::File::open(src)
                .await
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;

            let mut writer = object_store::buffered::BufWriter::new(
                self.store.clone(),
                self.path.clone(),
            );

            tokio::io::copy(&mut file, &mut writer)
                .await
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;

            writer
                .shutdown()
                .await
                .map_err(|e| CloudError::ThirdParty(Box::new(e)))?;

            Ok(())
        })
    }
}
