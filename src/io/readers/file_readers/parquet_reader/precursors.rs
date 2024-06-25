use super::ReadableParquetTable;

#[derive(Default, Debug, PartialEq)]
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
    fn update_from_parquet_file(&mut self, name: &str, field: String) {
        match name {
            "Id" => self.index = field.parse().unwrap_or_default(),
            "RetentionTime" => self.rt = field.parse().unwrap_or_default(),
            "MonoisotopicMz" => self.mz = field.parse().unwrap_or_default(),
            "Charge" => self.charge = field.parse().unwrap_or_default(),
            "Intensity" => self.intensity = field.parse().unwrap_or_default(),
            "ooK0" => self.im = field.parse().unwrap_or_default(),
            "MS1ParentFrameId" => {
                self.frame_index =
                    field.parse::<f32>().unwrap_or_default() as usize
            },
            "BinaryOffset" => self.offset = field.parse().unwrap_or_default(),
            "CollisionEnergy" => {
                self.collision_energy = field.parse().unwrap_or_default()
            },
            _ => {},
        }
    }
}
