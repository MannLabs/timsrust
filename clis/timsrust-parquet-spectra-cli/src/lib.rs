mod cli;
mod runner;

pub use cli::CLI;
pub use runner::run;
use serde::{Deserialize, Serialize};
use timsrust_core::io::formats::parquet::ParquetWriter;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub mz: f64,
    pub apex_intensity: u64,
}

// impl_parquet_scheme!(
//     Fragment,
//     [
//         (mz, arrow::datatypes::Float64Type, false),
//         (apex_intensity, arrow::datatypes::UInt64Type, false),
//     ]
// );

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Precursor {
    pub frame: u32,
    pub scan: u32,
    pub tof: u32,
    pub apex_intensity: u64,
    pub rt: f64,
    pub im: f64,
    pub mz: f64,
    pub start: u64,
    pub end: u64,
    pub charge: u8,
    pub index: u32,
    pub isolation_mz: f64,
    pub isolation_width: f64,
    pub ce: f64,
}

// impl_parquet_scheme!(
//     Precursor,
//     [
//         (frame, arrow::datatypes::UInt32Type, false),
//         (scan, arrow::datatypes::UInt32Type, false),
//         (tof, arrow::datatypes::UInt32Type, false),
//         (apex_intensity, arrow::datatypes::UInt64Type, false),
//         (rt, arrow::datatypes::Float64Type, false),
//         (im, arrow::datatypes::Float64Type, false),
//         (mz, arrow::datatypes::Float64Type, false),
//         (start, arrow::datatypes::UInt64Type, false),
//         (end, arrow::datatypes::UInt64Type, false),
//         (charge, arrow::datatypes::UInt8Type, false),
//         (index, arrow::datatypes::UInt32Type, false),
//         (isolation_mz, arrow::datatypes::Float64Type, false),
//         (isolation_width, arrow::datatypes::Float64Type, false),
//         (ce, arrow::datatypes::Float64Type, false),
//     ]
// );
