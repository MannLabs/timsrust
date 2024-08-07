use std::path::Path;
use timsrust::{
    ms_data::{Precursor, Spectrum},
    FileReader,
};

fn get_local_directory() -> &'static Path {
    Path::new(std::file!())
        .parent()
        .expect("Failed to get parent directory")
}

#[test]
fn minitdf_reader() {
    let file_name = "test2.ms2";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    let spectra: Vec<Spectrum> =
        FileReader::new(file_path).unwrap().read_all_spectra();
    let expected: Vec<Spectrum> = vec![
        Spectrum {
            mz_values: vec![100.0, 200.002, 300.03, 400.4],
            intensities: vec![1.0, 2.0, 3.0, 4.0],
            precursor: Precursor {
                mz: 123.4567,
                rt: 12.345,
                im: 1.234,
                charge: Some(1),
                intensity: Some(0.0),
                index: 1,
                frame_index: 1,
            },
            index: 1,
            collision_energy: 0.0,
            isolation_mz: 123.4567,
            isolation_width: 2.0,
        },
        Spectrum {
            mz_values: vec![1100.0, 1200.002, 1300.03, 1400.4],
            intensities: vec![10.0, 20.0, 30.0, 40.0],
            precursor: Precursor {
                mz: 987.6543,
                rt: 9.876,
                im: 0.9876,
                charge: Some(2),
                intensity: Some(0.0),
                index: 2,
                frame_index: 2,
            },
            index: 2,
            collision_energy: 0.0,
            isolation_mz: 987.6543,
            isolation_width: 3.0,
        },
    ];
    for i in 0..spectra.len() {
        assert_eq!(spectra[i], expected[i]);
    }
}

#[test]
fn tdf_reader_dda() {
    let file_name = "test.d";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    let spectra: Vec<Spectrum> =
        FileReader::new(file_path).unwrap().read_all_spectra();
    let expected: Vec<Spectrum> = vec![
        Spectrum {
            mz_values: vec![199.7633445943076],
            intensities: vec![162.0],
            precursor: Precursor {
                mz: 500.0,
                rt: 0.2,
                im: 1.25,
                charge: Some(2),
                intensity: Some(10.0),
                index: 1,
                frame_index: 1,
            },
            index: 0,
            collision_energy: 0.0,
            isolation_mz: 500.5,
            isolation_width: 2.0,
        },
        Spectrum {
            mz_values: vec![169.5419900362706, 695.6972509397959],
            intensities: vec![120.0, 624.0],
            precursor: Precursor {
                mz: 501.0,
                rt: 0.2,
                im: 1.0,
                charge: Some(3),
                intensity: Some(10.0),
                index: 2,
                frame_index: 1,
            },
            index: 1,
            collision_energy: 0.0,
            isolation_mz: 501.5,
            isolation_width: 2.0,
        },
        Spectrum {
            mz_values: vec![827.1915846690921],
            intensities: vec![714.0],
            precursor: Precursor {
                mz: 502.0,
                rt: 0.4,
                im: 1.25,
                charge: Some(2),
                intensity: Some(10.0),
                index: 3,
                frame_index: 3,
            },
            index: 2,
            collision_energy: 0.0,
            isolation_mz: 502.5,
            isolation_width: 2.0,
        },
    ];
    for i in 0..spectra.len() {
        assert_eq!(spectra[i], expected[i]);
    }
}
