use std::path::Path;
use timsrust::{AcquisitionType, FileReader, Frame, FrameType};

fn get_local_directory() -> &'static Path {
    Path::new(std::file!())
        .parent()
        .expect("Failed to get parent directory")
}

#[test]
fn tdf_reader_frames() {
    let file_name = "test.d";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    let frames: Vec<Frame> =
        FileReader::new(file_path).unwrap().read_all_frames();
    let expected: Vec<Frame> = vec![
        Frame {
            scan_offsets: vec![0, 1, 3, 6, 10],
            tof_indices: (0..10).collect(),
            intensities: (0..10).map(|x| (x + 1) * 2).collect(),
            index: 1,
            rt: 0.1,
            frame_type: FrameType::MS1,
        },
        Frame {
            scan_offsets: vec![0, 5, 11, 18, 26],
            tof_indices: (10..36).collect(),
            intensities: (10..36).map(|x| (x + 1) * 2).collect(),
            index: 2,
            rt: 0.2,
            frame_type: FrameType::MS2(AcquisitionType::DDAPASEF),
        },
        Frame {
            scan_offsets: vec![0, 9, 19, 30, 42],
            tof_indices: (36..78).collect(),
            intensities: (36..78).map(|x| (x + 1) * 2).collect(),
            index: 3,
            rt: 0.3,
            frame_type: FrameType::MS1,
        },
        Frame {
            scan_offsets: vec![0, 13, 27, 42, 58],
            tof_indices: (78..136).collect(),
            intensities: (78..136).map(|x| (x + 1) * 2).collect(),
            index: 4,
            rt: 0.4,
            frame_type: FrameType::MS2(AcquisitionType::DDAPASEF),
        },
    ];
    for i in 0..frames.len() {
        assert_eq!(&frames[i], &expected[i])
    }
}
