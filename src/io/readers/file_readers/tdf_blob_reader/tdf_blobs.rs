const U32_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TdfBlob {
    bytes: Vec<u8>,
}

impl TdfBlob {
    pub fn new(bytes: Vec<u8>) -> Self {
        assert!(bytes.len() % U32_SIZE == 0);
        Self { bytes }
    }

    pub fn get(&self, index: usize) -> u32 {
        assert!(index < self.len());
        Self::concatenate_bytes(
            self.bytes[index],
            self.bytes[index + self.len()],
            self.bytes[index + 2 * self.len()],
            self.bytes[index + 3 * self.len()],
        )
    }

    fn concatenate_bytes(b1: u8, b2: u8, b3: u8, b4: u8) -> u32 {
        b1 as u32
            | ((b2 as u32) << 8)
            | ((b3 as u32) << 16)
            | ((b4 as u32) << 24)
    }

    pub fn len(&self) -> usize {
        self.bytes.len() / U32_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
