/// An MS1 precursor that got selected for fragmentation.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Precursor {
    pub mz: f64,
    pub rt: f64,
    pub im: f64,
    pub charge: usize,
    pub intensity: f64,
    pub index: usize,
    pub frame_index: usize,
}

/// A type of quadrupole selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuadrupoleEvent {
    Precursor(Precursor),
    // Window(Window),
    // PrecursorList(Vec<Precursor>),
    None,
}

impl Default for QuadrupoleEvent {
    fn default() -> Self {
        Self::None
    }
}

impl QuadrupoleEvent {
    pub fn unwrap_as_precursor(&self) -> Precursor {
        match self {
            QuadrupoleEvent::Precursor(precursor) => *precursor,
            _ => {
                panic!("Not a precursor");
            },
        }
    }
}
