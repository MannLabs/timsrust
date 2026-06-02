#[macro_use]
mod macros;
mod charge;
mod converters;

use timsrust_utils::custom_error;

pub use charge::*;
pub use converters::*;

pub const PROTON_MASS: Mass = Mass(1.007276466812);

custom_error!(pub CoordinateError);

index_dimension!(FrameIndex);
value_dimension!(Rt);
impl ConvertibleTo<Rt> for FrameIndex {}
impl ConvertibleTo<FrameIndex> for Rt {}

index_dimension!(ScanIndex);
value_dimension!(Im);
impl ConvertibleTo<Im> for ScanIndex {}
impl ConvertibleTo<ScanIndex> for Im {}

index_dimension!(IntensityIndex);
value_dimension!(Intensity);
impl ConvertibleTo<Intensity> for IntensityIndex {}
impl ConvertibleTo<IntensityIndex> for Intensity {}

index_dimension!(TofIndex);
value_dimension!(Mz);
value_dimension!(Mass);
value_dimension!(Mh);
impl ConvertibleTo<Mz> for TofIndex {}
impl ConvertibleTo<TofIndex> for Mz {}

impl Mass {
    pub fn to_mh(&self) -> Mh {
        Mh(self.0 + PROTON_MASS.0)
    }

    pub fn to_mz(&self, charge: Charge) -> Mz {
        Mz(self.0 / i8::from(charge) as f64 + PROTON_MASS.0)
    }
}

impl Mh {
    pub fn to_mass(&self) -> Mass {
        Mass(self.0 - PROTON_MASS.0)
    }
}

impl Mz {
    pub fn to_mass(&self, charge: Charge) -> Mass {
        Mass((self.0 - PROTON_MASS.0) * (i8::from(charge) as f64))
    }
}

macro_rules! bit_conversion {
    (
        $index:ident, $value:ident
    ) => {
        impl Converter<$index, $value> for BitConverter {
            fn convert(&self, value: $index) -> $value {
                let bits = u32::from(value);
                $value::from(f32::from_bits(bits))
            }
        }

        impl Converter<$value, $index> for BitConverter {
            fn convert(&self, value: $value) -> $index {
                let bits = (f64::from(value) as f32).to_bits();
                $index::try_from(bits)
                    .expect("TofIndex conversion out of bounds")
            }
        }
    };
}

bit_conversion!(TofIndex, Mz);
bit_conversion!(ScanIndex, Im);
bit_conversion!(FrameIndex, Rt);
