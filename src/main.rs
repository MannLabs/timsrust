// use rayon::iter::ParallelIterator;
use std::env;
use timsrust::io::readers::FrameReader;
use timsrust::io::writers::MGFEntry;
use timsrust::ms_data::Frame;
use timsrust::{ms_data::Spectrum, FileReader};

fn quick_test() {
    // TODO move quick test out to separate program
    let args: Vec<String> = env::args().collect();
    let d_folder_name: &str = &args[1];
    let x = FileReader::new(d_folder_name.to_string()).unwrap();
    let spectrum_index: usize;
    if args.len() >= 3 {
        spectrum_index = args[2].parse().unwrap_or(0);
    } else {
        spectrum_index = 10;
    }
    let dda_spectra: Vec<Spectrum> = x.read_all_spectra();
    let spectrum = &dda_spectra[spectrum_index];
    // let spectrum: &Spectrum = &x.read_single_spectrum(spectrum_index);
    // // // println!("precursor {:?}", spectrum.precursor);
    // // // _ = MGFEntry::write_header(spectrum);
    println!("{}", MGFEntry::write(spectrum));
    // // // println!("{}", MGFEntry::write_header(spectrum));
    // // // println!("{}", MGFEntry::write_peaks(spectrum));
    // // // println!("mz values {:?}", spectrum.mz_values);
    // // // println!(
    // // //     "intensity values {:?}",
    // // //     spectrum.intensities
    // // // );
    // // // println!("{:?}", spectrum.as_mgf_entry());
    // // // MGFWriter::write_spectra(d_folder_name, &dda_spectra);
    // let frame = x.read_single_frame(2);
    let x = FrameReader::new(d_folder_name);
    // let frames: Vec<Frame> = x.parallel_filter(|x| x.msms_type != 0).collect();
    let frame: Frame = x.get(200);
    // let frame = &frames[200 - 2];
    println!("{:?}", frame);
}

fn main() {
    quick_test();
}
