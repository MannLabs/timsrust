use crate::io::readers::common::tdf_blobs::{TdfBlob, TdfBlobReader};

use crate::ms_data::{Frame, Spectrum};

pub trait ReadableFromBinFile {
    fn parse_from_ms_data_blob(buffer: TdfBlob, index: usize) -> Self;

    fn read_from_file(bin_file: &TdfBlobReader, index: usize) -> Self
    where
        Self: Sized,
    {
        let blob = bin_file.get_blob(index);
        Self::parse_from_ms_data_blob(blob, index)
    }
}

impl ReadableFromBinFile for Spectrum {
    fn parse_from_ms_data_blob(blob: TdfBlob, index: usize) -> Self {
        let mut spectrum: Spectrum = Spectrum::default();
        spectrum.index = index;
        if blob.len() == 0 {
            return spectrum;
        };
        let size: usize = blob.len();
        let mut spectrum_data: Vec<u32> = vec![0; size];
        for i in 0..size {
            spectrum_data[i] = blob.get(i)
        }
        let scan_count: usize = blob.len() / 3;
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
    fn parse_from_ms_data_blob(blob: TdfBlob, index: usize) -> Self {
        let mut frame = Frame::default();
        (frame.scan_offsets, frame.tof_indices, frame.intensities) =
            parse_frame(blob);
        frame.index = index;
        frame
    }
}

pub fn parse_frame(blob: TdfBlob) -> (Vec<usize>, Vec<u32>, Vec<u32>) {
    let mut tof_indices: Vec<u32> = vec![];
    let mut intensities: Vec<u32> = vec![];
    let mut scan_offsets: Vec<usize> = vec![];
    if blob.len() != 0 {
        let scan_count: usize = blob.get(0) as usize;
        let peak_count: usize = (blob.len() - scan_count) / 2;
        scan_offsets = read_scan_offsets(scan_count, peak_count, &blob);
        intensities = read_intensities(scan_count, peak_count, &blob);
        tof_indices =
            read_tof_indices(scan_count, peak_count, &blob, &scan_offsets);
    }
    (scan_offsets, tof_indices, intensities)
}

fn read_scan_offsets(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Vec<usize> {
    let mut scan_offsets: Vec<usize> = Vec::with_capacity(scan_count + 1);
    scan_offsets.push(0);
    for scan_index in 0..scan_count - 1 {
        let index = scan_index + 1;
        let scan_size: usize = (blob.get(index) / 2) as usize;
        scan_offsets.push(scan_offsets[scan_index] + scan_size);
    }
    scan_offsets.push(peak_count);
    scan_offsets
}

fn read_intensities(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
) -> Vec<u32> {
    let mut intensities: Vec<u32> = Vec::with_capacity(peak_count);
    for peak_index in 0..peak_count {
        let index: usize = scan_count + 1 + 2 * peak_index;
        intensities.push(blob.get(index));
    }
    intensities
}

fn read_tof_indices(
    scan_count: usize,
    peak_count: usize,
    blob: &TdfBlob,
    scan_offsets: &Vec<usize>,
) -> Vec<u32> {
    let mut tof_indices: Vec<u32> = Vec::with_capacity(peak_count);
    for scan_index in 0..scan_count {
        let start_offset: usize = scan_offsets[scan_index];
        let end_offset: usize = scan_offsets[scan_index + 1];
        let mut current_sum: u32 = 0;
        for peak_index in start_offset..end_offset {
            let index = scan_count + 2 * peak_index;
            let tof_index: u32 = blob.get(index);
            current_sum += tof_index;
            tof_indices.push(current_sum - 1);
        }
    }
    tof_indices
}
