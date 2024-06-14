use crate::{
    domain_converters::{Frame2RtConverter, Scan2ImConverter, Tof2MzConverter},
    file_readers::{
        common::sql_reader::{ReadableFromSql, SqlReader},
        ReadableFrames,
    },
    io::readers::frame_reader::FrameReader,
    ms_data::Frame,
};
use rayon::iter::ParallelIterator;

#[derive(Debug)]
pub struct TDFReader {
    frame_reader: FrameReader,
    pub tdf_sql_reader: SqlReader,
    pub rt_converter: Frame2RtConverter,
    pub im_converter: Scan2ImConverter,
    pub mz_converter: Tof2MzConverter,
}

impl TDFReader {
    pub fn new(path: &String) -> Self {
        let tdf_sql_reader: SqlReader = SqlReader {
            path: String::from(path),
        };
        let frame_reader: FrameReader = FrameReader::new(&path);
        Self {
            rt_converter: Frame2RtConverter::from_sql(&tdf_sql_reader),
            im_converter: Scan2ImConverter::from_sql(&tdf_sql_reader),
            mz_converter: Tof2MzConverter::from_sql(&tdf_sql_reader),
            tdf_sql_reader: tdf_sql_reader,
            frame_reader: frame_reader,
        }
    }
}

impl ReadableFrames for TDFReader {
    fn read_single_frame(&self, index: usize) -> Frame {
        self.frame_reader.get(index)
    }

    fn read_all_frames(&self) -> Vec<Frame> {
        self.frame_reader.parallel_filter(|_| true).collect()
    }

    fn read_all_ms1_frames(&self) -> Vec<Frame> {
        self.frame_reader
            .parallel_filter(|x| x.msms_type == 0)
            .collect()
    }

    fn read_all_ms2_frames(&self) -> Vec<Frame> {
        self.frame_reader
            .parallel_filter(|x| x.msms_type != 0)
            .collect()
    }
}
