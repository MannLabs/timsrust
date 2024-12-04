use crate::Spectrum;
use std::{fs::File, io::Write, path::Path};

pub struct MGFWriter;

impl MGFWriter {
    pub fn write_spectra(input_file_path: &str, spectra: &Vec<Spectrum>) {
        let output_file_path = {
            let input_path = Path::new(&input_file_path);
            let file_stem =
                Path::new(&input_file_path).file_stem().unwrap_or_default();
            let new_file_name = format!("{}.mgf", file_stem.to_string_lossy());
            input_path.with_file_name(new_file_name)
        };
        let mut file =
            File::create(output_file_path).expect("Failed to create file");
        for spectrum in spectra {
            _ = file.write_all("BEGIN IONS\n".as_bytes());
            _ = file.write_all(MGFEntry::write(spectrum).as_bytes());
            _ = file.write_all("END IONS\n".as_bytes());
        }
        file.flush().expect("Failed to flush to file");
    }
}

pub struct MGFEntry;

impl MGFEntry {
    pub fn write_header(spectrum: &Spectrum) -> String {
        // TODO
        let precursor = spectrum.precursor.unwrap();
        let title = precursor.index;
        let intensity = precursor.intensity.unwrap_or(0.0);
        let charge = precursor.charge.unwrap_or(0);
        let ms2_data = format!(
            "TITLE=index:{}, im:{:.4}, intensity:{:.4}, frame:{}, ce:{:.4}\nPEPMASS={:.4}\nCHARGE={}\nRTINSECONDS={:.2}\n",
            title, precursor.im, intensity, precursor.frame_index, spectrum.collision_energy, precursor.mz, charge, precursor.rt
        );
        ms2_data
    }

    pub fn write_peaks(spectrum: &Spectrum) -> String {
        let mut ms2_data: String = String::new();
        for (mz, intensity) in
            spectrum.mz_values.iter().zip(spectrum.intensities.iter())
        {
            ms2_data.push_str(&format!("{:.4}\t{:.0}\n", mz, intensity));
        }
        ms2_data
    }

    pub fn write(spectrum: &Spectrum) -> String {
        format!(
            "{}{}",
            Self::write_header(spectrum),
            Self::write_peaks(spectrum)
        )
    }
}
