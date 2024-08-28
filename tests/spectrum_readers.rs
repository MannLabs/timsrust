use std::path::Path;
#[cfg(feature = "tdf")]
use timsrust::readers::{
    FrameWindowSplittingConfiguration, QuadWindowExpansionStrategy,
};
use timsrust::{
    readers::{SpectrumProcessingParams, SpectrumReader, SpectrumReaderConfig},
    Precursor, Spectrum,
};

fn get_local_directory() -> &'static Path {
    Path::new(std::file!())
        .parent()
        .expect("Failed to get parent directory")
}

#[cfg(feature = "minitdf")]
#[test]
fn minitdf_reader() {
    let file_name = "test2.ms2";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    let spectra: Vec<Result<Spectrum, _>> = SpectrumReader::build()
        .with_path(file_path)
        .finalize()
        .unwrap()
        .get_all();
    let expected: Vec<Spectrum> = vec![
        Spectrum {
            mz_values: vec![100.0, 200.002, 300.03, 400.4],
            intensities: vec![1.0, 2.0, 3.0, 4.0],
            precursor: Some(Precursor {
                mz: 123.4567,
                rt: 12.345,
                im: 1.234,
                charge: Some(1),
                intensity: Some(0.0),
                index: 1,
                frame_index: 1,
            }),
            index: 1,
            collision_energy: 0.0,
            isolation_mz: 123.4567,
            isolation_width: 2.0,
        },
        Spectrum {
            mz_values: vec![1100.0, 1200.002, 1300.03, 1400.4],
            intensities: vec![10.0, 20.0, 30.0, 40.0],
            precursor: Some(Precursor {
                mz: 987.6543,
                rt: 9.876,
                im: 0.9876,
                charge: Some(2),
                intensity: Some(0.0),
                index: 2,
                frame_index: 2,
            }),
            index: 2,
            collision_energy: 0.0,
            isolation_mz: 987.6543,
            isolation_width: 3.0,
        },
    ];
    for (i, spectrum) in spectra.into_iter().enumerate() {
        assert_eq!(spectrum.unwrap(), expected[i]);
    }
}

#[cfg(feature = "tdf")]
#[test]
fn tdf_reader_dda() {
    let file_name = "test.d";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    let spectra: Vec<Result<Spectrum, _>> = SpectrumReader::build()
        .with_path(file_path)
        .finalize()
        .unwrap()
        .get_all();
    let expected: Vec<Spectrum> = vec![
        Spectrum {
            mz_values: vec![199.7633445943076],
            intensities: vec![162.0],
            precursor: Some(Precursor {
                mz: 500.0,
                rt: 0.2,
                im: 1.25,
                charge: Some(2),
                intensity: Some(10.0),
                index: 1,
                frame_index: 1,
            }),
            index: 0,
            collision_energy: 0.0,
            isolation_mz: 500.5,
            isolation_width: 2.0,
        },
        Spectrum {
            mz_values: vec![169.5419900362706, 695.6972509397959],
            intensities: vec![120.0, 624.0],
            precursor: Some(Precursor {
                mz: 501.0,
                rt: 0.2,
                im: 1.0,
                charge: Some(3),
                intensity: Some(10.0),
                index: 2,
                frame_index: 1,
            }),
            index: 1,
            collision_energy: 0.0,
            isolation_mz: 501.5,
            isolation_width: 2.0,
        },
        Spectrum {
            mz_values: vec![827.1915846690921],
            intensities: vec![714.0],
            precursor: Some(Precursor {
                mz: 502.0,
                rt: 0.4,
                im: 1.25,
                charge: Some(2),
                intensity: Some(10.0),
                index: 3,
                frame_index: 3,
            }),
            index: 2,
            collision_energy: 0.0,
            isolation_mz: 502.5,
            isolation_width: 2.0,
        },
    ];
    for (i, spectrum) in spectra.into_iter().enumerate() {
        assert_eq!(spectrum.unwrap(), expected[i]);
    }
}

#[cfg(feature = "tdf")]
#[test]
fn test_dia_even() {
    let file_name = "dia_test.d";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    for i in 1..3 {
        let spectra = SpectrumReader::build()
            .with_path(&file_path)
            .with_config(SpectrumReaderConfig {
                frame_splitting_params:
                    FrameWindowSplittingConfiguration::Quadrupole(
                        QuadWindowExpansionStrategy::Even(i),
                    ),
                spectrum_processing_params: SpectrumProcessingParams::default(),
            })
            .finalize()
            .unwrap()
            .get_all();
        // 4 frames, 2 windows in each, i splits/window
        assert_eq!(spectra.len(), 4 * 2 * i);
    }
}

#[cfg(feature = "tdf")]
#[test]
fn test_dia_uniform_mobility() {
    let file_name = "dia_test.d";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    for i in [0.02, 0.05, 0.1] {
        let spectra = SpectrumReader::build()
            .with_path(&file_path)
            .with_config(SpectrumReaderConfig {
                frame_splitting_params:
                    FrameWindowSplittingConfiguration::Window(
                        QuadWindowExpansionStrategy::UniformMobility(
                            (i, i),
                            None,
                        ),
                    ),
                spectrum_processing_params: SpectrumProcessingParams::default(),
            })
            .finalize()
            .unwrap()
            .get_all();
        for f in spectra.iter() {
            println!("i={} -> {:?}", i, f.as_ref().unwrap().precursor);
        }
        // Not all frames have scan windows from 0.5 to 1.5 ... so ... I need to think
        // on how to express this in the test
        assert!(spectra.len() >= (1.0 / i) as usize);

        // 4 frames, each split in 1.0/i chunks max, 1.0 is the IMS width of a frame
        // but not all frames span windows in that range
        assert!(spectra.len() < 4 * (1.0 / i) as usize,);

        // TODO make a more accurate test where we measure the differences in ion
        // mobilities and see if they are within the expected range
    }
}

#[cfg(feature = "tdf")]
#[test]
fn test_dia_uniform_scans() {
    let file_name = "dia_test.d";
    let file_path = get_local_directory()
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string();
    for i in [20, 100, 200] {
        let spectra = SpectrumReader::build()
            .with_path(&file_path)
            .with_config(SpectrumReaderConfig {
                frame_splitting_params:
                    FrameWindowSplittingConfiguration::Window(
                        QuadWindowExpansionStrategy::UniformScan((i, i)),
                    ),
                spectrum_processing_params: SpectrumProcessingParams::default(),
            })
            .finalize()
            .unwrap()
            .get_all();
        for f in spectra.iter() {
            println!("i={} -> {:?}", i, f.as_ref().unwrap().precursor);
        }

        // Since there are 709 scans in the test data ... we can expect
        // the number of breaks to be (709 / i) + 1  ... if we had a single
        // window that spanned the entire scan range.
        // ... A more strict test would filter for each frame index and
        // within each make sure the number matches the ratio ... here I am
        // Just checking the overall number.
        const NUM_FRAMES: usize = 4;
        const NUM_SCANS: usize = 709;

        assert!(spectra.len() >= (NUM_SCANS / i) as usize + 1);
        assert!(spectra.len() < NUM_FRAMES * (NUM_SCANS / i) as usize + 1);
    }
}
