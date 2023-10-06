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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrecursorType {
    Precursor(Precursor),
    // Window(Window),
    // PrecursorList(Vec<Precursor>),
    None,
}

impl Default for PrecursorType {
    fn default() -> Self {
        Self::None
    }
}

impl PrecursorType {
    pub fn unwrap_as_precursor(&self) -> Precursor {
        match self {
            PrecursorType::Precursor(precursor) => *precursor,
            _ => {
                panic!("Not a precursor");
            },
        }
    }
}
