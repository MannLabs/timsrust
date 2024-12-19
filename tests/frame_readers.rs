#[cfg(feature = "tdf")]
mod tests {
    use std::{path::Path, sync::Arc};
    use timsrust::{
        readers::FrameReader, AcquisitionType, Frame, MSLevel,
        QuadrupoleSettings,
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
            .unwrap()
            .get_all_ms1()
            .into_iter()
            .map(|x| x.unwrap())
            .collect();
        let expected: Vec<Frame> = vec![
            Frame {
                scan_offsets: vec![0, 1, 3, 6, 10],
                tof_indices: (0..10).collect(),
                intensities: (0..10).map(|x| (x + 1) * 2).collect(),
                index: 1,
                rt_in_seconds: 0.1,
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
                rt_in_seconds: 0.3,
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
            .unwrap()
            .get_all_ms2()
            .into_iter()
            .map(|x| x.unwrap())
            .collect();
        let expected: Vec<Frame> = vec![
            // Frame::default(),
            Frame {
                scan_offsets: vec![0, 5, 11, 18, 26],
                tof_indices: (10..36).collect(),
                intensities: (10..36).map(|x| (x + 1) * 2).collect(),
                index: 2,
                rt_in_seconds: 0.2,
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
                rt_in_seconds: 0.4,
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

    #[test]
    fn tdf_reader_frames_dia() {
        let file_name = "dia_test.d";
        let file_path = get_local_directory()
            .join(file_name)
            .to_str()
            .unwrap()
            .to_string();
        let frames: Vec<Frame> = FrameReader::new(&file_path)
            .unwrap()
            .get_all_ms2()
            .into_iter()
            .map(|x| x.unwrap())
            .collect();

        assert_eq!(frames.len(), 4);
        for i in 0..frames.len() {
            assert_eq!(frames[i].scan_offsets.len(), 710);
            assert_eq!(frames[i].scan_offsets[0], 0);
            assert_eq!(
                frames[i].scan_offsets.last().unwrap(),
                &frames[i].intensities.len()
            );
            assert_eq!(
                frames[i].tof_indices.len(),
                frames[i].intensities.len()
            );
        }
        assert_eq!(&frames[0].tof_indices[0], &251695u32);
        assert_eq!(&frames[0].intensities[0], &503392u32);
        assert_eq!(&frames[0].tof_indices.len(), &754376);
        assert_eq!(&frames[0].intensities.len(), &754376);

        assert_eq!(&frames[1].tof_indices[0], &1006071u32);
        assert_eq!(&frames[1].intensities[0], &2012144u32);
        assert_eq!(&frames[1].tof_indices.len(), &1257057);
        assert_eq!(&frames[1].intensities.len(), &1257057);

        assert_eq!(&frames[2].tof_indices[0], &4022866u32);
        assert_eq!(&frames[2].intensities[0], &8045734u32);
        assert_eq!(&frames[2].tof_indices.len(), &2262419);
        assert_eq!(&frames[2].intensities.len(), &2262419);

        assert_eq!(&frames[3].tof_indices[0], &6285285u32);
        assert_eq!(&frames[3].intensities[0], &12570572u32);
        assert_eq!(&frames[3].tof_indices.len(), &2765100);
        assert_eq!(&frames[3].intensities.len(), &2765100);
    }
}
