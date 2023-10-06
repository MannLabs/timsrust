use rayon::prelude::*;

use crate::{
    converters::{ConvertableIndex, Scan2ImConverter},
    file_readers::{
        common::sql_reader::{
            PasefFrameMsMsTable, PrecursorTable, ReadableFromSql,
        },
        frame_readers::tdf_reader::TDFReader,
    },
    vec_utils::argsort,
    Precursor,
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
        let pasef_frames: PasefFrameMsMsTable =
            PasefFrameMsMsTable::from_sql(&tdf_reader.tdf_sql_reader);
        let im_reader: Scan2ImConverter = tdf_reader.im_converter;
        let precursor_table: PrecursorTable =
            PrecursorTable::from_sql(&tdf_reader.tdf_sql_reader);
        let retention_times: Vec<f64> = tdf_reader.frame_table.rt.clone();
        let precursors: Vec<Precursor> = (0..precursor_table.mz.len())
            .into_par_iter()
            .map(|index| {
                let frame_id: usize = precursor_table.precursor_frame[index];
                let scan_id: f64 = precursor_table.scan_average[index];
                Precursor {
                    mz: precursor_table.mz[index],
                    rt: retention_times[frame_id],
                    im: im_reader.convert(scan_id),
                    charge: precursor_table.charge[index],
                    intensity: precursor_table.intensity[index],
                    index: index + 1, //TODO?
                    frame_index: frame_id,
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
