use crate::vec_utils::counts_to_indptr;

pub fn parse_frame(data: Vec<u32>) -> (Vec<u64>, Vec<u32>, Vec<u32>) {
    let mut tof_indices: Vec<u32> = vec![];
    let mut intensities: Vec<u32> = vec![];
    let mut scan_offsets: Vec<u64> = vec![];
    if data.len() != 0 {
        let scan_count: usize = read_scan_count(&data);
        let scan_counts: Vec<u32> = read_scan_counts(scan_count, &data);
        tof_indices = read_tof_indices(scan_count, &data, &scan_counts);
        intensities = read_intensities(scan_count, &data);
        scan_offsets = counts_to_indptr(scan_counts);
    }
    (scan_offsets, tof_indices, intensities)
}

fn read_scan_count(data: &Vec<u32>) -> usize {
    let scan_count = data[0] as usize;
    scan_count
}

fn read_scan_counts(scan_count: usize, data: &Vec<u32>) -> Vec<u32> {
    let mut scan_counts: Vec<u32> = data[..scan_count].to_vec();
    let ion_count: u32 = ((data.len() - scan_count) / 2) as u32;
    let mut defined_scan_counts: u32 = 0;
    for i in &scan_counts[1..] {
        defined_scan_counts += i / 2
    }
    let last_scan: u32 = ion_count - defined_scan_counts;
    // println!("{:} {:}, {:}", last_scan, ion_count, defined_scan_counts);
    scan_counts.rotate_left(1);
    scan_counts[scan_count - 1] = last_scan;
    for i in 0..scan_counts.len() - 1 {
        scan_counts[i] /= 2;
    }
    scan_counts
}

fn read_tof_indices(
    scan_count: usize,
    data: &Vec<u32>,
    scan_counts: &Vec<u32>,
) -> Vec<u32> {
    let mut tof_indices: Vec<u32> =
        data.iter().skip(scan_count).step_by(2).cloned().collect();
    let mut index: usize = 0;
    for size in scan_counts {
        let mut current_sum: u32 = 0;
        for _i in 0..*size {
            current_sum += tof_indices[index];
            tof_indices[index] = current_sum;
            index += 1;
        }
    }
    for i in 0..tof_indices.len() {
        tof_indices[i] -= 1;
    }
    tof_indices
}

fn read_intensities(scan_count: usize, data: &Vec<u32>) -> Vec<u32> {
    let intensities: Vec<u32> = data
        .iter()
        .skip(scan_count + 1)
        .step_by(2)
        .cloned()
        .collect();
    intensities
}
