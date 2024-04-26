use rayon::prelude::*;

use crate::{
    domain_converters::ConvertableDomain,
    file_readers::{
        common::sql_reader::{
            PasefFrameMsMsTable, PrecursorTable, ReadableFromSql,
        },
        frame_readers::tdf_reader::TDFReader,
    },
    ms_data::Precursor,
    utils::vec_utils::argsort,
};

#[derive(Debug)]
pub struct PrecursorReader {
    pub precursors: Vec<Precursor>,
    pub pasef_frames: PasefFrameMsMsTable,
    pub order: Vec<usize>,
    pub offsets: Vec<usize>,
    pub count: usize,
}

impl PrecursorReader {
    pub fn new(tdf_reader: &TDFReader) -> Self {
        let select_collision_energy_sql = String::from(
            "SELECT CollisionEnergy FROM PasefFrameMsMsInfo GROUP BY Precursor",
        );
        let pasef_frames: PasefFrameMsMsTable =
            PasefFrameMsMsTable::from_sql(&tdf_reader.tdf_sql_reader);
        let precursor_table: PrecursorTable =
            PrecursorTable::from_sql(&tdf_reader.tdf_sql_reader);
        // let retention_times: Vec<f64> = tdf_reader.frame_table.rt.clone();
        let collision_energies = tdf_reader
            .tdf_sql_reader
            .get_data_from_sql(&select_collision_energy_sql);
        let precursors: Vec<Precursor> = (0..precursor_table.mz.len())
            .into_par_iter()
            .map(|index| {
                let frame_id: usize = precursor_table.precursor_frame[index];
                let scan_id: f64 = precursor_table.scan_average[index];
                Precursor {
                    mz: precursor_table.mz[index],
                    rt: tdf_reader.rt_converter.convert(frame_id as u32),
                    im: tdf_reader.im_converter.convert(scan_id),
                    charge: precursor_table.charge[index],
                    intensity: precursor_table.intensity[index],
                    index: index + 1, //TODO?
                    frame_index: frame_id,
                    collision_energy: collision_energies[index],
                }
            })
            .collect();
        let order: Vec<usize> = argsort(&pasef_frames.precursor);
        let count: usize = *pasef_frames.precursor.iter().max().unwrap();
        let mut offsets: Vec<usize> = Vec::with_capacity(count + 1);
        offsets.push(0);
        for (offset, &index) in order.iter().enumerate().take(order.len() - 1) {
            let second_index: usize = order[offset + 1];
            if pasef_frames.precursor[index]
                != pasef_frames.precursor[second_index]
            {
                offsets.push(offset + 1)
            }
        }
        offsets.push(order.len());
        Self {
            precursors,
            pasef_frames,
            order,
            offsets,
            count,
        }
    }
}
