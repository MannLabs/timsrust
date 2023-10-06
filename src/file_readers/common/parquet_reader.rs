use parquet::file::reader::{FileReader, SerializedFileReader};
use std::fs::File;

use crate::Precursor;

pub fn read_parquet_precursors(
    parquet_file_name: &String,
) -> (Vec<Precursor>, Vec<u64>) {
    let file: File = File::open(parquet_file_name).unwrap();
    let reader: SerializedFileReader<File> =
        SerializedFileReader::new(file).unwrap();
    let mut precursors: Vec<Precursor> = vec![];
    let mut offsets: Vec<u64> = vec![];
    for record in reader.get_row_iter(None).unwrap() {
        let mut precursor: Precursor = Precursor::default();
        for (name, field) in record.get_column_iter() {
            match name.to_string().as_str() {
                "Id" => precursor.index = field.to_string().parse().unwrap(),
                "RetentionTime" => {
                    precursor.rt = field.to_string().parse().unwrap()
                },
                "MonoisotopicMz" => {
                    precursor.mz = field.to_string().parse().unwrap_or(0.0)
                },
                "Charge" => {
                    precursor.charge =
                        field.to_string().parse().unwrap_or(0.0) as usize
                },
                "Intensity" => {
                    precursor.intensity = field.to_string().parse().unwrap()
                },
                "ooK0" => precursor.im = field.to_string().parse().unwrap(),
                "MS1ParentFrameId" => {
                    precursor.frame_index =
                        field.to_string().parse::<f32>().unwrap() as usize
                },
                "BinaryOffset" => {
                    offsets.push(field.to_string().parse().unwrap())
                },
                _ => {},
            }
        }
        precursors.push(precursor);
    }
    (precursors, offsets)
}
