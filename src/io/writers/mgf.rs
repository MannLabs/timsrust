use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::ms_data::Spectrum;

pub struct MGFWriter {}

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
            _ = file.write_all(spectrum.as_mgf_header().as_bytes());
            _ = file.write_all(spectrum.as_mgf_peaks().as_bytes());
            _ = file.write_all("END IONS\n".as_bytes());
        }
        file.flush().expect("Failed to flush to file");
    }
}

pub trait MGFFormat {
    fn as_mgf_header(&self) -> String;

    fn as_mgf_peaks(&self) -> String;
}

impl MGFFormat for Spectrum {
    fn as_mgf_header(&self) -> String {
        let precursor = self.precursor;
        let title = precursor.index;
        let ms2_data = format!(
            "TITLE=index:{}, im:{:.4}, intensity:{:.4}, frame:{}, ce:{:.4}\nPEPMASS={:.4}\nCHARGE={}\nRT={:.2}\n",
            title, precursor.im, precursor.intensity, precursor.frame_index, precursor.collision_energy, precursor.mz, precursor.charge, precursor.rt
        );
        ms2_data
    }

    fn as_mgf_peaks(&self) -> String {
        let mut ms2_data: String = String::new();
        for (mz, intensity) in
            self.mz_values.iter().zip(self.intensities.iter())
        {
            ms2_data.push_str(&format!("{:.4}\t{:.0}\n", mz, intensity));
        }
        ms2_data
    }
}
