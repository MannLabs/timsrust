/// The kind of acquisition that was used.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AcquisitionType {
    DDAPASEF,
    DIAPASEF,
    DiagonalDIAPASEF,
    PRMPASEF,
    Unknown,
}

impl Default for AcquisitionType {
    fn default() -> Self {
        Self::Unknown
    }
}
