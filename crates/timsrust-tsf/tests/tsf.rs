use std::fs;
use std::path::PathBuf;
use timsrust_core::utils::reader::Reader;
use timsrust_tsf::TSFSpectrumReader;

fn get_test_folder(file_name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(file_name)
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn tsf_reader() {
    let file_dir = get_test_folder("test_tsf.d");
    let file_path =
        fs::canonicalize(file_dir).expect("missing test_tsf.d folder");
    let file_path = file_path.to_string_lossy().into_owned();
    let reader = TSFSpectrumReader::new(&file_path).unwrap();
    assert_eq!(
        reader.len(),
        255,
        "TSF dataset should expose one spectrum per frame"
    );
    let spectrum = reader.get(0).expect("failed to read first TSF spectrum");
    assert_eq!(spectrum.tof_indices().len(), 15636);
    assert_eq!(spectrum.intensities().len(), 15636);
    let spectrum2 = reader.get(1).expect("failed to read second TSF spectrum");
    assert_eq!(spectrum2.tof_indices().len(), 14304);
    assert_eq!(spectrum2.intensities().len(), 14304);
}
