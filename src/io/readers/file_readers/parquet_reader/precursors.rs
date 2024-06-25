use parquet::record::Field;

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
    fn update_from_parquet_file(&mut self, name: &String, field: &Field) {
        match name.to_string().as_str() {
            "Id" => self.index = field.to_string().parse().unwrap_or_default(),
            "RetentionTime" => {
                self.rt = field.to_string().parse().unwrap_or_default()
            },
            "MonoisotopicMz" => {
                self.mz = field.to_string().parse().unwrap_or_default()
            },
            "Charge" => {
                self.charge = field.to_string().parse().unwrap_or_default()
            },
            "Intensity" => {
                self.intensity = field.to_string().parse().unwrap_or_default()
            },
            "ooK0" => self.im = field.to_string().parse().unwrap_or_default(),
            "MS1ParentFrameId" => {
                self.frame_index =
                    field.to_string().parse::<f32>().unwrap_or_default()
                        as usize
            },
            "BinaryOffset" => {
                self.offset = field.to_string().parse().unwrap_or_default()
            },
            "CollisionEnergy" => {
                self.collision_energy =
                    field.to_string().parse().unwrap_or_default()
            },
            _ => {},
        }
    }
}
