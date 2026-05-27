use timsrust_utils::{custom_error, enumerated_error};

use crate::CoordinateError;

custom_error!(pub SpectrumError);
custom_error!(pub FrameError);
custom_error!(pub PrecursorError);
custom_error!(pub IonError);

enumerated_error!(
    pub TimsError,
    Spectrum(SpectrumError),
    Frame(FrameError),
    Precursor(PrecursorError),
    Ion(IonError),
    SparseVec(timsrust_utils::vec::SparseVecError),
    NDArray(timsrust_utils::ndarray::NDArrayError),
    Coordinate(CoordinateError)
);

pub type TimsResult<T> = Result<T, TimsError>;
