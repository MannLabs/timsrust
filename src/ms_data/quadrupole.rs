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
