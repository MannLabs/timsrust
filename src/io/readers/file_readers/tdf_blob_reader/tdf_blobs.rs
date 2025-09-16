use zstd::stream::copy_decode;

const BLOB_TYPE_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Clone, Debug, PartialEq)]
pub struct TdfBlob {
    bytes: Vec<u8>,
}

impl TdfBlob {
    pub(crate) fn try_new(bytes: Vec<u8>) -> Result<Self, TdfBlobError> {
        Self::check_len(&bytes)?;
        Ok(Self { bytes })
    }

    pub(crate) fn decompress_reset(
        &mut self,
        compressed_bytes: &[u8],
    ) -> Result<(), TdfBlobError> {
        self.bytes.clear();
        copy_decode(compressed_bytes, &mut self.bytes)
            .map_err(|_| TdfBlobError::Decompression)?;
        Self::check_len(self.bytes.as_slice())?;
        Ok(())
    }

    pub fn new_empty() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    fn check_len(bytes: &[u8]) -> Result<(), TdfBlobError> {
        if bytes.len() % BLOB_TYPE_SIZE != 0 {
            Err(TdfBlobError::Size(bytes.len()))
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "minitdf")]
    pub(crate) fn get_all(&self) -> Vec<u32> {
        (0..self.len())
            .map(|index| self.get(index).expect(
                "When iterating over the length of a tdf blob, you cannot go out of bounds"
            ))
            .collect()
    }

    pub(crate) fn get(&self, index: usize) -> Option<u32> {
        if index >= self.len() {
            None
        } else {
            Some(Self::concatenate_bytes(
                self.bytes[index],
                self.bytes[index + self.len()],
                self.bytes[index + 2 * self.len()],
                self.bytes[index + 3 * self.len()],
            ))
        }
    }

    fn concatenate_bytes(b1: u8, b2: u8, b3: u8, b4: u8) -> u32 {
        b1 as u32
            | ((b2 as u32) << 8)
            | ((b3 as u32) << 16)
            | ((b4 as u32) << 24)
    }

    pub fn len(&self) -> usize {
        self.bytes.len() / BLOB_TYPE_SIZE
    }

    #[cfg(feature = "minitdf")]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TdfBlobError {
    #[error("Length {0} is not a multiple of {BLOB_TYPE_SIZE}")]
    Size(usize),
    #[error("Decompression fails")]
    Decompression,
}
