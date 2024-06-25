use std::env;
use timsrust::io::writers::MGFEntry;
use timsrust::{ms_data::Spectrum, FileReader};

fn quick_test() {
    let args: Vec<String> = env::args().collect();
    let d_folder_name: &str = &args[1];
    let x = FileReader::new(d_folder_name.to_string()).unwrap();
    let dda_spectra: Vec<Spectrum> = x.read_all_spectra();
    let spectrum_index: usize;
    if args.len() >= 3 {
        spectrum_index = args[2].parse().unwrap_or(0);
    } else {
        spectrum_index = 10;
    }
    println!("precursor {:?}", dda_spectra[spectrum_index].precursor);
    _ = MGFEntry::write_header(&dda_spectra[spectrum_index]);
    // println!(
    //     "precursor\n{:?}",
    //     dda_spectra[spectrum_index].as_mgf_header()
    // );
    println!("mz values {:?}", dda_spectra[spectrum_index].mz_values);
    println!(
        "intensity values {:?}",
        dda_spectra[spectrum_index].intensities
    );
    // println!("{:?}", dda_spectra[spectrum_index].as_mgf_entry());
    // MGFWriter::write_spectra(d_folder_name, &dda_spectra);
}

fn main() {
    quick_test();
}
