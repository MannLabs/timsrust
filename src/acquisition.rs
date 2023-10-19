/// The kind of acquisition that was used.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AcquisitionType {
    DDAPASEF,
    DIAPASEF,
    Unknown,
}
