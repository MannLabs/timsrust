use std::{
    fs::File,
    io::{BufWriter, Write},
};

use timsrust_core::{Mz, Spectrum};

pub struct MGFWriter {
    file: BufWriter<File>,
}

impl MGFWriter {
    pub fn new(output_path: &str) -> Self {
        let file = File::create(output_path).expect("Failed to create file");
        let writer = BufWriter::new(file);
        Self { file: writer }
    }

    pub fn write(&mut self, spectrum: &Spectrum<Mz>) {
        self.file
            .write_all("BEGIN IONS\n".as_bytes())
            .expect("Failed to write BEGIN IONS");
        self.file
            .write_all(MGFEntry::write(spectrum).as_bytes())
            .expect("Failed to write entry");
        self.file
            .write_all("END IONS\n".as_bytes())
            .expect("Failed to write END IONS");
        self.file.flush().expect("Failed to flush to file");
    }
}

pub(crate) struct MGFEntry;

impl MGFEntry {
    pub(crate) fn write_header<C>(spectrum: &Spectrum<C>) -> String {
        // TODO
        let precursor = spectrum.precursor().as_ref().unwrap();
        let title = precursor.index();
        let intensity = precursor.intensity().unwrap_or(0.0);
        let charge = precursor.charge().map(i8::from).unwrap_or(0);
        let ms2_data = format!(
            "TITLE=index:{}, im:{:.4}, intensity:{:.4}, frame:{}, ce:{:.4}, width:{:.4}\nPEPMASS={:.4}\nCHARGE={}\nRTINSECONDS={:.2}\n",
            title,
            precursor.im(),
            intensity,
            precursor.frame_index(),
            spectrum.isolation_window().collision_energy(),
            spectrum.isolation_window().width(),
            precursor.mz(),
            charge,
            precursor.rt()
        );
        ms2_data
    }

    pub(crate) fn write_peaks(spectrum: &Spectrum<Mz>) -> String {
        let capacity = spectrum.len() * 16; // Estimate capacity for mz and intensity pairs
        let mut ms2_data: String = String::with_capacity(capacity);
        for (mz, intensity) in spectrum
            .mz_values()
            .iter()
            .zip(spectrum.intensities().iter())
        {
            ms2_data.push_str(&format!("{:.4}\t{:.0}\n", mz, intensity));
        }
        ms2_data
    }

    pub(crate) fn write(spectrum: &Spectrum<Mz>) -> String {
        format!(
            "{}{}",
            Self::write_header(spectrum),
            Self::write_peaks(spectrum)
        )
    }
}
