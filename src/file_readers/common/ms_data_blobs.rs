mod parsers;

use std::fs::File;

use memmap2::Mmap;
use zstd::decode_all;

use crate::{Frame, Spectrum};

use self::parsers::parse_frame;

#[derive(Debug, Default)]
pub struct BinFileReader {
    file_offsets: Vec<u64>,
    mmap: Option<Mmap>,
}

impl BinFileReader {
    pub fn new(file_name: String, file_offsets: Vec<u64>) -> Self {
        let tdf_bin_file: File = File::open(&file_name)
            .expect("File cannot be opened. Is the path correct?");
        let mmap: Option<Mmap> =
            Some(unsafe { Mmap::map(&tdf_bin_file).unwrap() });
        Self { file_offsets, mmap }
    }

    fn read_blob(&self, index: usize) -> Vec<u8> {
        let offset: u64 = self.file_offsets[index as usize];
        if let Some(mmap) = self.mmap.as_ref() {
            let raw_byte_count: &[u8] =
                &mmap[offset as usize..(offset + 4) as usize];
            let byte_count: u32 =
                u32::from_le_bytes(raw_byte_count.try_into().unwrap());
            let compressed_blob: &[u8] = &mmap
                [(offset + 8) as usize..offset as usize + byte_count as usize];
            let blob: Vec<u8> = decode_all(compressed_blob).unwrap();
            return blob;
        };
        return vec![0];
    }

    pub fn size(&self) -> usize {
        self.file_offsets.len()
    }
}

pub trait ReadableFromBinFile {
    fn parse_from_ms_data_blob(buffer: Vec<u8>, index: usize) -> Self;

    fn read_from_file(bin_file: &BinFileReader, index: usize) -> Self
    where
        Self: Sized,
    {
        let blob: Vec<u8> = bin_file.read_blob(index);
        Self::parse_from_ms_data_blob(blob, index)
    }
}

impl ReadableFromBinFile for Spectrum {
    fn parse_from_ms_data_blob(blob: Vec<u8>, index: usize) -> Self {
        let mut spectrum: Spectrum = Spectrum::default();
        spectrum.index = index;
        if blob.len() == 0 {
            return spectrum;
        };
        let size: usize = blob.len() / std::mem::size_of::<u32>();
        let first: &[u8] = &blob[0 * size..1 * size];
        let second: &[u8] = &blob[1 * size..2 * size];
        let third: &[u8] = &blob[2 * size..3 * size];
        let fourth: &[u8] = &blob[3 * size..4 * size];
        let mut spectrum_data: Vec<u32> = vec![0; size];
        for i in 0..size {
            spectrum_data[i] = first[i] as u32;
            spectrum_data[i] |= (second[i] as u32) << 8;
            spectrum_data[i] |= (third[i] as u32) << 16;
            spectrum_data[i] |= (fourth[i] as u32) << 24;
        }
        let scan_count: usize = blob.len() / 3 / std::mem::size_of::<u32>();
        let tof_indices_bytes: &[u32] =
            &spectrum_data[..scan_count as usize * 2];
        let intensities_bytes: &[u32] =
            &spectrum_data[scan_count as usize * 2..];
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
    fn parse_from_ms_data_blob(blob: Vec<u8>, index: usize) -> Self {
        let mut frame = Frame::default();
        (frame.scan_offsets, frame.tof_indices, frame.intensities) =
            parse_frame(blob);
        frame.index = index;
        frame
    }
}
