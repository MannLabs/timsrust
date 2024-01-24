fn get_u32_from_blob(blob: &Vec<u8>, index: usize) -> u32 {
    let size: usize = blob.len() / std::mem::size_of::<u32>();
    return concatenate_four_bytes_into_u32(
        blob[index],
        blob[size + index],
        blob[2 * size + index],
        blob[3 * size + index],
    );
}

fn concatenate_four_bytes_into_u32(b1: u8, b2: u8, b3: u8, b4: u8) -> u32 {
    (b1 as u32) | ((b2 as u32) << 8) | ((b3 as u32) << 16) | ((b4 as u32) << 24)
}

pub fn parse_frame(blob: Vec<u8>) -> (Vec<u64>, Vec<u32>, Vec<u32>) {
    let mut tof_indices: Vec<u32> = vec![];
    let mut intensities: Vec<u32> = vec![];
    let mut scan_offsets: Vec<u64> = vec![];
    if blob.len() != 0 {
        let scan_count: usize = get_u32_from_blob(&blob, 0) as usize;
        let peak_count: u32 =
            ((blob.len() / std::mem::size_of::<u32>() - scan_count) / 2) as u32;
        scan_offsets = read_scan_offsets(scan_count, peak_count, &blob);
        intensities = read_intensities(scan_count, peak_count, &blob);
        tof_indices =
            read_tof_indices(scan_count, peak_count, &blob, &scan_offsets);
    }
    (scan_offsets, tof_indices, intensities)
}

fn read_scan_offsets(
    scan_count: usize,
    peak_count: u32,
    blob: &Vec<u8>,
) -> Vec<u64> {
    // let mut scan_offsets: Vec<u64> = vec![0; scan_count + 1];
    let mut scan_offsets: Vec<u64> = Vec::with_capacity(scan_count + 1);
    scan_offsets.push(0);
    for scan_index in 0..scan_count - 1 {
        let scan_size = (get_u32_from_blob(blob, scan_index + 1) / 2) as u64;
        scan_offsets.push(scan_offsets[scan_index] + scan_size);
    }
    scan_offsets.push(peak_count as u64);
    scan_offsets
}

fn read_intensities(
    scan_count: usize,
    peak_count: u32,
    blob: &Vec<u8>,
) -> Vec<u32> {
    let mut intensities: Vec<u32> = Vec::with_capacity(peak_count as usize);
    for i in 0..peak_count {
        intensities
            .push(get_u32_from_blob(blob, scan_count + 1 + 2 * i as usize));
    }
    intensities
}

fn read_tof_indices(
    scan_count: usize,
    peak_count: u32,
    blob: &Vec<u8>,
    scan_offsets: &Vec<u64>,
) -> Vec<u32> {
    let mut tof_indices: Vec<u32> = Vec::with_capacity(peak_count as usize);
    for scan_index in 0..scan_count {
        let start_offset = scan_offsets[scan_index] as usize;
        let end_offset = scan_offsets[scan_index + 1] as usize;
        let mut current_sum: u32 = 0;
        for i in start_offset..end_offset {
            let tof_index = get_u32_from_blob(blob, scan_count + 2 * i);
            current_sum += tof_index;
            tof_indices.push(current_sum - 1);
        }
    }
    tof_indices
}
