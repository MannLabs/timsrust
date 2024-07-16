const BLOB_TYPE_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TdfBlob {
    bytes: Vec<u8>,
}

impl TdfBlob {
    pub fn new(bytes: Vec<u8>) -> Result<Self, TdfBlobError> {
        if bytes.len() % BLOB_TYPE_SIZE != 0 {
            Err(TdfBlobError::InvalidLength {
                length: bytes.len(),
            })
        } else {
            Ok(Self { bytes })
        }
    }

    pub fn get(&self, index: usize) -> Result<u32, TdfBlobError> {
        if index >= self.len() {
            Err(TdfBlobError::IndexOutOfBounds {
                length: self.len(),
                index,
            })
        } else {
            Ok(Self::concatenate_bytes(
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TdfBlobError {
    #[error("Length {length} not a multiple of {BLOB_TYPE_SIZE}")]
    InvalidLength { length: usize },
    #[error("Index {index} out of bounds for length {length})")]
    IndexOutOfBounds { index: usize, length: usize },
}
