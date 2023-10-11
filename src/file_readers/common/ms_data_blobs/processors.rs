use std::io::{prelude::*, BufReader};
use zstd::stream::read::Decoder;

use super::readers::{MSDataBlob, MSDataBlobReader, MSDataBlobState};

pub struct MSDataBlobProcessor {
    ms_data_blob: MSDataBlob,
}

impl MSDataBlobProcessor {
    pub fn from_file(path: &String, offset: u64) -> Vec<u32> {
        let ms_data_blob: MSDataBlob =
            MSDataBlobReader::new(path, offset).read();
        Self { ms_data_blob }.decompress().byte_shuffle_and_unpack()
    }

    fn decompress(mut self) -> Self {
        if self.ms_data_blob.data.len() != 0 {
            let reader: &[u8] = &self.ms_data_blob.data;
            let mut decoder: Decoder<BufReader<&[u8]>> = Decoder::new(reader)
                .expect("Cannot set decoder. Are the bytes correct?");
            let mut buf: Vec<u8> = Vec::new();
            decoder
                .read_to_end(&mut buf)
                .expect("Cannot decompress bytes. Are they zstd compressed?");
            self.ms_data_blob.data = buf;
        }
        self.ms_data_blob.state = MSDataBlobState::Decompressed;
        self
    }

    fn byte_shuffle_and_unpack(&self) -> Vec<u32> {
        let size: usize = self.ms_data_blob.data.len() / 4;
        let first: &[u8] = &self.ms_data_blob.data[0 * size..1 * size];
        let second: &[u8] = &self.ms_data_blob.data[1 * size..2 * size];
        let third: &[u8] = &self.ms_data_blob.data[2 * size..3 * size];
        let fourth: &[u8] = &self.ms_data_blob.data[3 * size..4 * size];
        let mut frame_data: Vec<u32> = vec![0; size];
        for i in 0..size {
            frame_data[i as usize] = first[i as usize] as u32;
            frame_data[i as usize] += (second[i as usize] as u32) << 8;
            frame_data[i as usize] += (third[i as usize] as u32) << 16;
            frame_data[i as usize] += (fourth[i as usize] as u32) << 24;
        }
        frame_data
    }
}
