use std::path::Path;

use rayon::prelude::*;

use crate::{
    domain_converters::{
        ConvertableDomain, Frame2RtConverter, Scan2ImConverter,
    },
    io::readers::{
        file_readers::sql_reader::{
            pasef_frame_msms::SqlPasefFrameMsMs, precursors::SqlPrecursor,
            ReadableSqlTable, SqlReader,
        },
        metadata_reader::MetadataReader,
    },
    ms_data::Precursor,
    utils::vec_utils::argsort,
};

#[derive(Debug)]
pub struct PrecursorReader {
    pub precursors: Vec<Precursor>,
    pub pasef_frames: Vec<SqlPasefFrameMsMs>,
    pub order: Vec<usize>,
    pub offsets: Vec<usize>,
    pub count: usize,
}

impl PrecursorReader {
    pub fn new(path: &String) -> Self {
        let metadata = MetadataReader::new(&path);
        let rt_converter: Frame2RtConverter = metadata.rt_converter;
        let im_converter: Scan2ImConverter = metadata.im_converter;
        let tdf_sql_reader =
            SqlReader::open(Path::new(path).join("analysis.tdf")).unwrap();
        let pasef_frames =
            SqlPasefFrameMsMs::from_sql_reader(&tdf_sql_reader).unwrap();
        let precursors =
            SqlPrecursor::from_sql_reader(&tdf_sql_reader).unwrap();
        let precursors: Vec<Precursor> = (0..precursors.len())
            .into_par_iter()
            .map(|index| {
                let frame_id: usize = precursors[index].precursor_frame;
                let scan_id: f64 = precursors[index].scan_average;
                Precursor {
                    mz: precursors[index].mz,
                    rt: rt_converter.convert(frame_id as u32),
                    im: im_converter.convert(scan_id),
                    charge: precursors[index].charge,
                    intensity: precursors[index].intensity,
                    index: index + 1, //TODO?
                    frame_index: frame_id,
                    // TODO OPTIMIZE!!!!!
                    collision_energy: pasef_frames
                        .iter()
                        .find(|&x| x.precursor == index + 1)
                        .unwrap()
                        .collision_energy,
                }
            })
            .collect();
        let pasef_precursors =
            &pasef_frames.iter().map(|x| x.precursor).collect();
        let order: Vec<usize> = argsort(&pasef_precursors);
        let count: usize = *pasef_precursors.iter().max().unwrap();
        let mut offsets: Vec<usize> = Vec::with_capacity(count + 1);
        offsets.push(0);
        for (offset, &index) in order.iter().enumerate().take(order.len() - 1) {
            let second_index: usize = order[offset + 1];
            if pasef_precursors[index] != pasef_precursors[second_index] {
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
