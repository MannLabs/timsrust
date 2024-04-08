pub mod acquisition;
pub mod frames;
pub mod precursors;
pub mod spectra;

pub use acquisition::AcquisitionType;
pub use frames::{Frame, FrameType};
pub use precursors::{Precursor, QuadrupoleEvent};
pub use spectra::Spectrum;
