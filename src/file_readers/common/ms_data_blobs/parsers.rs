const U32_SIZE: usize = std::mem::size_of::<u32>();

#[inline(always)]
fn get_u32_from_blob(blob: &Vec<u8>, index: usize) -> u32 {
    let size: usize = blob.len() / U32_SIZE;
    return concatenate_four_bytes_into_u32(
        blob[index],
        blob[size + index],
        blob[2 * size + index],
        blob[3 * size + index],
    );
}

#[inline(always)]
fn concatenate_four_bytes_into_u32(b1: u8, b2: u8, b3: u8, b4: u8) -> u32 {
    (b1 as u32) | ((b2 as u32) << 8) | ((b3 as u32) << 16) | ((b4 as u32) << 24)
}

pub fn parse_frame(blob: Vec<u8>) -> (Vec<usize>, Vec<u32>, Vec<u32>) {
    let mut tof_indices: Vec<u32> = vec![];
    let mut intensities: Vec<u32> = vec![];
    let mut scan_offsets: Vec<usize> = vec![];
    if blob.len() != 0 {
        let scan_count: usize = get_u32_from_blob(&blob, 0) as usize;
        let peak_count: usize = (blob.len() / U32_SIZE - scan_count) / 2;
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
    blob: &Vec<u8>,
) -> Vec<usize> {
    let mut scan_offsets: Vec<usize> = Vec::with_capacity(scan_count + 1);
    scan_offsets.push(0);
    for scan_index in 0..scan_count - 1 {
        let index = scan_index + 1;
        let scan_size: usize = (get_u32_from_blob(blob, index) / 2) as usize;
        scan_offsets.push(scan_offsets[scan_index] + scan_size);
    }
    scan_offsets.push(peak_count);
    scan_offsets
}

fn read_intensities(
    scan_count: usize,
    peak_count: usize,
    blob: &Vec<u8>,
) -> Vec<u32> {
    let mut intensities: Vec<u32> = Vec::with_capacity(peak_count);
    for peak_index in 0..peak_count {
        let index: usize = scan_count + 1 + 2 * peak_index;
        intensities.push(get_u32_from_blob(blob, index));
    }
    intensities
}

fn read_tof_indices(
    scan_count: usize,
    peak_count: usize,
    blob: &Vec<u8>,
    scan_offsets: &Vec<usize>,
) -> Vec<u32> {
    let mut tof_indices: Vec<u32> = Vec::with_capacity(peak_count);
    for scan_index in 0..scan_count {
        let start_offset: usize = scan_offsets[scan_index];
        let end_offset: usize = scan_offsets[scan_index + 1];
        let mut current_sum: u32 = 0;
        for peak_index in start_offset..end_offset {
            let index = scan_count + 2 * peak_index;
            let tof_index: u32 = get_u32_from_blob(blob, index);
            current_sum += tof_index;
            tof_indices.push(current_sum - 1);
        }
    }
    tof_indices
}
