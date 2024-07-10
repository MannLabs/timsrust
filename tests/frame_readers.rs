use rayon::iter::ParallelIterator;
use std::{path::Path, sync::Arc};
use timsrust::{
    io::readers::FrameReader,
    ms_data::{AcquisitionType, Frame, MSLevel, QuadrupoleSettings},
};

fn get_local_directory() -> &'static Path {
    Path::new(std::file!())
        .parent()
        .expect("Failed to get parent directory")
}

#[test]
fn tdf_reader_frames1() {
    let file_name = "test.d";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    let frames: Vec<Frame> = FrameReader::new(&file_path)
        .parallel_filter(|x| x.msms_type == 0)
        .collect();
    let expected: Vec<Frame> = vec![
        Frame {
            scan_offsets: vec![0, 1, 3, 6, 10],
            tof_indices: (0..10).collect(),
            intensities: (0..10).map(|x| (x + 1) * 2).collect(),
            index: 1,
            rt: 0.1,
            ms_level: MSLevel::MS1,
            quadrupole_settings: Arc::new(QuadrupoleSettings::default()),
            acquisition_type: AcquisitionType::DDAPASEF,
            intensity_correction_factor: 1.0 / 100.0,
            window_group: 0,
        },
        // Frame::default(),
        Frame {
            scan_offsets: vec![0, 9, 19, 30, 42],
            tof_indices: (36..78).collect(),
            intensities: (36..78).map(|x| (x + 1) * 2).collect(),
            index: 3,
            rt: 0.3,
            ms_level: MSLevel::MS1,
            quadrupole_settings: Arc::new(QuadrupoleSettings::default()),
            acquisition_type: AcquisitionType::DDAPASEF,
            intensity_correction_factor: 1.0 / 100.0,
            window_group: 0,
        },
        // Frame::default(),
    ];
    for i in 0..expected.len() {
        assert_eq!(&frames[i], &expected[i])
    }
}

#[test]
fn tdf_reader_frames2() {
    let file_name = "test.d";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    let frames: Vec<Frame> = FrameReader::new(&file_path)
        .parallel_filter(|x| x.msms_type != 0)
        .collect();
    let expected: Vec<Frame> = vec![
        // Frame::default(),
        Frame {
            scan_offsets: vec![0, 5, 11, 18, 26],
            tof_indices: (10..36).collect(),
            intensities: (10..36).map(|x| (x + 1) * 2).collect(),
            index: 2,
            rt: 0.2,
            ms_level: MSLevel::MS2,
            quadrupole_settings: Arc::new(QuadrupoleSettings::default()),
            acquisition_type: AcquisitionType::DDAPASEF,
            intensity_correction_factor: 1.0 / 100.0,
            window_group: 0,
        },
        // Frame::default(),
        Frame {
            scan_offsets: vec![0, 13, 27, 42, 58],
            tof_indices: (78..136).collect(),
            intensities: (78..136).map(|x| (x + 1) * 2).collect(),
            index: 4,
            rt: 0.4,
            ms_level: MSLevel::MS2,
            quadrupole_settings: Arc::new(QuadrupoleSettings::default()),
            acquisition_type: AcquisitionType::DDAPASEF,
            intensity_correction_factor: 1.0 / 100.0,
            window_group: 0,
        },
    ];
    for i in 0..expected.len() {
        assert_eq!(&frames[i], &expected[i])
    }
}

// TODO test for DIA
