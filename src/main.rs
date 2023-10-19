use std::env;
use timsrust::{FileReader, Spectrum};

fn main() {
    let args: Vec<String> = env::args().collect();
    let d_folder_name: &str = &args[1];
    let x = FileReader::new(d_folder_name.to_string()).unwrap();
    let dda_spectra: Vec<Spectrum> = x.read_all_spectra();
    let precursor_index: usize;
    if args.len() >= 3 {
        precursor_index = args[2].parse().unwrap_or(0);
    } else {
        precursor_index = 1000;
    }

    println!("precursor {:?}", dda_spectra[precursor_index].precursor);
    println!(
        "precursor {:?}",
        dda_spectra[precursor_index].mz_values.len()
    );
    println!(
        "precursor {:?}",
        dda_spectra[precursor_index].intensities.len()
    );
    println!("precursor {:?}", dda_spectra[precursor_index].mz_values);
    println!("precursor {:?}", dda_spectra[precursor_index].intensities);
}
