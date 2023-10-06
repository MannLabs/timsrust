use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    fs::File,
    io::{prelude::*, SeekFrom},
};

#[derive(Debug)]
pub struct MSDataBlob {
    pub data: Vec<u8>,
    pub state: MSDataBlobState,
}

#[derive(Debug)]
pub enum MSDataBlobState {
    Unprocessed,
    Decompressed,
}

impl Default for MSDataBlobState {
    fn default() -> Self {
        Self::Unprocessed
    }
}

#[derive(Debug)]
pub struct MSDataBlobReader {
    tdf_bin_file: File,
    offset: u64,
}

impl MSDataBlobReader {
    pub fn new(path: &String, offset: u64) -> Self {
        let tdf_bin_file: File = Self::open_file(path);
        let reader: MSDataBlobReader = Self {
            tdf_bin_file,
            offset,
        };
        reader
    }

    fn open_file(path: &String) -> File {
        File::open(path).expect("File cannot be opened. Is the path correct?")
    }

    pub fn read(&mut self) -> MSDataBlob {
        self.reset_binary_offset();
        let mut byte_count: u32 = self.read_byte_count();
        let _scan_count: u32 = self.read_scan_count();
        byte_count -= 8;
        let ms_data: Vec<u8> = self.read_compressed_bytes(byte_count);
        MSDataBlob {
            data: ms_data,
            state: MSDataBlobState::default(),
        }
    }

    fn reset_binary_offset(&mut self) {
        let pos: SeekFrom = SeekFrom::Start(self.offset);
        self.tdf_bin_file
            .seek(pos)
            .expect("Offset cannot be seeked. Is it in range?");
    }

    fn read_byte_count(&mut self) -> u32 {
        self.tdf_bin_file
            .read_u32::<LittleEndian>()
            .expect("Cannot read byte count, is it little endian?")
    }

    fn read_scan_count(&mut self) -> u32 {
        self.tdf_bin_file
            .read_u32::<LittleEndian>()
            .expect("Cannot read scan count, is it little endian?")
    }

    fn read_compressed_bytes(&mut self, byte_count: u32) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![0; byte_count as usize];
        self.tdf_bin_file
            .read_exact(&mut buf)
            .expect("Cannot read compressed bytes. Are the offset and byte count correct?");
        buf
    }
}
