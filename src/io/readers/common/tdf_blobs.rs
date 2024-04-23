use memmap2::Mmap;
use std::fs::File;
use std::io;
use zstd::decode_all;

const U32_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Debug, Default)]
pub struct TdfBlob {
    bytes: Vec<u8>,
}

impl TdfBlob {
    pub fn get(&self, index: usize) -> u32 {
        Self::concatenate_bytes(
            self.bytes[index],
            self.bytes[index + self.len()],
            self.bytes[index + 2 * self.len()],
            self.bytes[index + 3 * self.len()],
        )
    }

    #[inline(always)]
    fn concatenate_bytes(b1: u8, b2: u8, b3: u8, b4: u8) -> u32 {
        b1 as u32
            | ((b2 as u32) << 8)
            | ((b3 as u32) << 16)
            | ((b4 as u32) << 24)
    }

    pub fn len(&self) -> usize {
        self.bytes.len() / U32_SIZE
    }
}

#[derive(Debug)]
pub struct TdfBlobReader {
    file_name: String,
    file_offsets: Vec<usize>,
    mmap: Mmap,
}

impl TdfBlobReader {
    pub fn new(
        file_name: String,
        file_offsets: Vec<usize>,
    ) -> Result<Self, io::Error> {
        let file: File = File::open(&file_name)?;
        let mmap: Mmap = unsafe { Mmap::map(&file)? };
        Ok(Self {
            file_name,
            file_offsets,
            mmap,
        })
    }

    pub fn get_blob(&self, index: usize) -> TdfBlob {
        if index >= self.len() {
            return TdfBlob::default();
        }
        let offset: usize = self.file_offsets[index as usize];
        let byte_count: u32 = self.get_byte_count(offset);
        if byte_count <= 8 {
            return TdfBlob::default();
        }
        let compressed_bytes: &[u8] = &self.mmap
            [(offset + 8) as usize..offset as usize + byte_count as usize];
        match decode_all(compressed_bytes) {
            Ok(bytes) => TdfBlob { bytes },
            Err(_) => TdfBlob::default(),
        }
    }

    pub fn get_file_name(self) -> String {
        self.file_name
    }

    fn get_byte_count(&self, offset: usize) -> u32 {
        let raw_byte_count: &[u8] =
            &self.mmap[offset as usize..(offset + 4) as usize];
        u32::from_le_bytes(raw_byte_count.try_into().unwrap())
    }

    pub fn len(&self) -> usize {
        self.file_offsets.len()
    }
}
