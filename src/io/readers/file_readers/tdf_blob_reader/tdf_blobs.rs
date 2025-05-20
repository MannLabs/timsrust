const BLOB_TYPE_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TdfBlob {
    bytes: Vec<u8>,
    shuffled: bool,
}

impl TdfBlob {
    pub fn new(bytes: Vec<u8>, shuffled: bool) -> Result<Self, TdfBlobError> {
        if bytes.len() % BLOB_TYPE_SIZE != 0 {
            Err(TdfBlobError(bytes.len()))
        } else {
            Ok(Self { bytes, shuffled })
        }
    }

    #[cfg(feature = "minitdf")]
    pub fn get_all(&self) -> Vec<u32> {
        (0..self.len())
            .map(|index| self.get(index).expect(
                "When iterating over the length of a tdf blob, you cannot go out of bounds"
            ))
            .collect()
    }

    pub fn get(&self, index: usize) -> Option<u32> {
        if index >= self.len() {
            None
        } else {
            if self.shuffled {
                Some(Self::concatenate_bytes(
                    self.bytes[index],
                    self.bytes[index + self.len()],
                    self.bytes[index + 2 * self.len()],
                    self.bytes[index + 3 * self.len()],
                ))
            } else {
                let index = index * BLOB_TYPE_SIZE;
                Some(Self::concatenate_bytes(
                    self.bytes[index],
                    self.bytes[index + 1],
                    self.bytes[index + 2],
                    self.bytes[index + 3],
                ))
            }
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
#[error("Length {0} is not a multiple of {BLOB_TYPE_SIZE}")]
pub struct TdfBlobError(usize);
