/// The kind of acquisition that was used.
#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum AcquisitionType {
    DDAPASEF,
    DIAPASEF,
    DiagonalDIAPASEF,
    // PRMPASEF,
    /// Default value.
    #[default]
    Unknown,
}
