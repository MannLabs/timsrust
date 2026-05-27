/// The kind of acquisition that was used.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AcquisitionType {
    DDAPASEF,
    DIAPASEF,
    DiagonalDIAPASEF,
    // PRMPASEF,
    /// Default value.
    #[default]
    Unknown,
}
