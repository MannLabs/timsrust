// use rayon::prelude::*;
use timsrust::SpectrumReader;
use timsrust::core::utils::thread::Synced;
use timsrust_cli_core::prelude::*;
use timsrust_mgf::MGFWriter;

fn runner(
    in_path: impl AsRef<str>,
    out_path: impl AsRef<str>,
    min_spectrum_size: usize,
    top_n: usize,
) {
    let time = std::time::Instant::now();
    let spectrum_reader = SpectrumReader::new(in_path).unwrap();
    let mgf_writer = MGFWriter::new(out_path.as_ref());
    let synced_mgf_writer = Synced::from(mgf_writer);
    let spec_count = spectrum_reader
        .par_iter()
        .filter_map(|spectrum| {
            #[allow(clippy::collapsible_if)]
            if let Ok(spectrum) = spectrum {
                let spectrum = spectrum.get_top_n(top_n);
                if spectrum.len() >= min_spectrum_size {
                    let _ = synced_mgf_writer
                        .with_lock(|mgf_writer| mgf_writer.write(&spectrum));
                    return Some(1);
                }
            }
            None
        })
        .sum::<usize>();
    log::info!(
        "Wrote {} spectra to {} in {:?}",
        spec_count,
        out_path.as_ref(),
        time.elapsed()
    );
}

fn main() {
    let matches = read_args();
    let _ctx = init_from_matches(&matches);
    let in_path: String = matches
        .get_one::<String>("input")
        .and_then(|s| s.parse().ok())
        .expect("Invalid input path");
    let out_path: String = matches
        .get_one::<String>("output")
        .and_then(|s| s.parse().ok())
        .expect("Invalid output path");
    let min_spectrum_size = matches
        .get_one::<String>("min-spectrum-size")
        .and_then(|s| s.parse::<usize>().ok())
        .expect("Invalid min_spectrum_size");
    let top_n = matches
        .get_one::<String>("top-n")
        .and_then(|s| s.parse::<usize>().ok())
        .expect("Invalid min_spectrum_size");
    runner(in_path, out_path, min_spectrum_size, top_n);
}

fn read_args() -> ArgMatches {
    base_command("timsrust-spectrum-filter")
        .arg(
            Arg::new("input")
                .required(true)
                .index(1)
                .help("Input path (.spec.parquet will be replaced with .precursors.parquet and .fragments.parquet)"),
        )
        .arg(
            Arg::new("output")
                .required(true)
                .index(2)
                .help("Output path for filtered spectra"),
        )
        .arg(
            Arg::new("min-spectrum-size")
                .long("min-spectrum-size")
                .default_value("5")
                .help("Minimum number of peaks in filtered spectra"),
        )
        .arg(
            Arg::new("top-n")
                .long("top-n")
                .default_value("0")
                .help("Keep top n peaks only (0 = all)"),
        )
        .get_matches()
}
