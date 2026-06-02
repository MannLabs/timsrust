use timsrust_core::{Converter, Mz, TofIndex};

pub mod parquet_path;
pub mod precursor_reader;
pub mod spectrum_reader;

#[derive(Debug, Clone)]
pub struct Tof2MzConverter();

impl Converter<TofIndex, Mz> for Tof2MzConverter {
    fn convert(&self, value: TofIndex) -> Mz {
        let bits = u32::from(value);
        Mz::from(f32::from_bits(bits))
    }
}

impl Converter<Mz, TofIndex> for Tof2MzConverter {
    fn convert(&self, value: Mz) -> TofIndex {
        let bits = (f64::from(value) as f32).to_bits();
        TofIndex::try_from(bits).expect("TofIndex conversion out of bounds")
    }
}
