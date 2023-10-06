mod parsers;
mod processors;
mod readers;

use crate::{Frame, Spectrum};

use self::{parsers::parse_frame, processors::MSDataBlobProcessor};

#[derive(Debug, Default, Clone)]
pub struct BinFileReader {
    file_name: String,
    file_offsets: Vec<u64>,
}

impl BinFileReader {
    pub fn new(file_name: String, file_offsets: Vec<u64>) -> Self {
        Self {
            file_name,
            file_offsets,
        }
    }

    fn read_blob(&self, index: usize) -> Vec<u32> {
        let offset: u64 = self.file_offsets[index as usize];
        MSDataBlobProcessor::from_file(&self.file_name, offset)
    }

    pub fn size(&self) -> usize {
        self.file_offsets.len()
    }
}

pub trait ReadableFromBinFile {
    fn parse_from_ms_data_blob(buffer: Vec<u32>, index: usize) -> Self;

    fn read_from_file(bin_file: &BinFileReader, index: usize) -> Self
    where
        Self: Sized,
    {
        let buffer: Vec<u32> = bin_file.read_blob(index);
        Self::parse_from_ms_data_blob(buffer, index)
    }
}

impl ReadableFromBinFile for Spectrum {
    fn parse_from_ms_data_blob(buffer: Vec<u32>, index: usize) -> Self {
        let mut spectrum: Spectrum = Spectrum::default();
        spectrum.index = index;
        if buffer.len() == 0 {
            return spectrum;
        };
        let scan_count: usize = buffer.len() / 3;
        let tof_indices_bytes: &[u32] = &buffer[..scan_count as usize * 2];
        let intensities_bytes: &[u32] = &buffer[scan_count as usize * 2..];
        let mz_values: &[f64] =
            bytemuck::cast_slice::<u32, f64>(tof_indices_bytes);
        let intensity_values: &[f32] =
            bytemuck::cast_slice::<u32, f32>(intensities_bytes);
        spectrum.intensities =
            intensity_values.iter().map(|&x| x as f64).collect();
        spectrum.mz_values = mz_values.to_vec();
        spectrum
    }
}

impl ReadableFromBinFile for Frame {
    fn parse_from_ms_data_blob(buffer: Vec<u32>, index: usize) -> Self {
        let mut frame = Frame::default();
        (frame.scan_offsets, frame.tof_indices, frame.intensities) =
            parse_frame(buffer);
        frame.index = index;
        frame
    }
}
