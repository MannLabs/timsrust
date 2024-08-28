use super::ReadableParquetTable;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParquetPrecursor {
    pub mz: f64,
    pub rt: f64,
    pub im: f64,
    pub charge: usize,
    pub intensity: f64,
    pub index: usize,
    pub frame_index: usize,
    pub offset: u64,
    pub collision_energy: f64,
}

impl ReadableParquetTable for ParquetPrecursor {
    fn update_from_parquet_file(&mut self, key: &str, value: String) {
        match key {
            "Id" => self.index = Self::parse_default_field(value),
            "RetentionTime" => self.rt = Self::parse_default_field(value),
            "MonoisotopicMz" => self.mz = Self::parse_default_field(value),
            "Charge" => self.charge = Self::parse_default_field(value),
            "Intensity" => self.intensity = Self::parse_default_field(value),
            "ooK0" => self.im = Self::parse_default_field(value),
            "MS1ParentFrameId" => {
                self.frame_index = Self::parse_default_field(value)
            },
            "BinaryOffset" => self.offset = Self::parse_default_field(value),
            "CollisionEnergy" => {
                self.collision_energy = Self::parse_default_field(value)
            },
            _ => {},
        }
    }
}
