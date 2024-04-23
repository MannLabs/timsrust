use {
    crate::{
        file_readers::{
            common::{
                ms_data_blobs::ReadableFromBinFile,
                sql_reader::{FrameTable, ReadableFromSql, SqlReader},
            },
            ReadableFrames,
        },
        io::readers::common::tdf_blobs::TdfBlobReader,
        AcquisitionType, ConvertableDomain, Frame, Frame2RtConverter, MSLevel,
        QuadrupoleSettings, Scan2ImConverter, Tof2MzConverter,
    },
    rayon::prelude::*,
    std::{path::Path, sync::Arc},
};

#[derive(Debug)]
pub struct TDFReader {
    pub path: String,
    pub tdf_sql_reader: SqlReader,
    tdf_bin_reader: TdfBlobReader,
    pub rt_converter: Frame2RtConverter,
    pub im_converter: Scan2ImConverter,
    pub mz_converter: Tof2MzConverter,
    pub frame_table: FrameTable,
    pub acquisition: AcquisitionType,
    ms_levels: Vec<MSLevel>,
}

impl TDFReader {
    pub fn new(path: &String) -> Self {
        let tdf_sql_reader: SqlReader = SqlReader {
            path: String::from(path),
        };
        let frame_table: FrameTable = FrameTable::from_sql(&tdf_sql_reader);
        let file_name: String = Path::new(&path)
            .join("analysis.tdf_bin")
            .to_string_lossy()
            .to_string();
        let tdf_bin_reader: TdfBlobReader = TdfBlobReader::new(
            String::from(&file_name),
            frame_table.offsets.iter().map(|x| *x as usize).collect(),
        )
        .unwrap();
        let ms_levels: Vec<MSLevel> = frame_table
            .msms_type
            .iter()
            .map(|msms_type| match msms_type {
                0 => MSLevel::MS1,
                8 => MSLevel::MS2,
                9 => MSLevel::MS2,
                _ => MSLevel::Unknown,
            })
            .collect();
        let mut acquisition = AcquisitionType::Unknown;
        if frame_table.msms_type.contains(&8) {
            acquisition = AcquisitionType::DDAPASEF;
        } else if frame_table.msms_type.contains(&9) {
            acquisition = AcquisitionType::DIAPASEF;
        }
        Self {
            path: path.to_string(),
            tdf_bin_reader: tdf_bin_reader,
            rt_converter: Self::get_rt_converter(&frame_table),
            im_converter: Scan2ImConverter::from_sql(&tdf_sql_reader),
            mz_converter: Tof2MzConverter::from_sql(&tdf_sql_reader),
            frame_table: frame_table,
            tdf_sql_reader: tdf_sql_reader,
            ms_levels: ms_levels,
            acquisition: acquisition,
        }
    }

    fn get_rt_converter(frame_table: &FrameTable) -> Frame2RtConverter {
        let retention_times: Vec<f64> = frame_table.rt.clone();
        Frame2RtConverter::from_values(retention_times)
    }
}

impl ReadableFrames for TDFReader {
    fn read_single_frame(&self, index: usize) -> Frame {
        let mut frame: Frame =
            Frame::read_from_file(&self.tdf_bin_reader, index);
        frame.rt = self.rt_converter.convert(index as u32);
        frame.index = self.frame_table.id[index];
        frame.ms_level = self.ms_levels[index];
        frame.acquisition_type = self.acquisition;
        if frame.ms_level == MSLevel::MS2 {
            frame.quadrupole_settings = Arc::new(QuadrupoleSettings::default());
        }
        frame
    }

    fn read_all_frames(&self) -> Vec<Frame> {
        (0..self.tdf_bin_reader.len())
            .into_par_iter()
            .map(|index| self.read_single_frame(index))
            .collect()
    }

    fn read_all_ms1_frames(&self) -> Vec<Frame> {
        (0..self.tdf_bin_reader.len())
            .into_par_iter()
            .map(|index| match self.ms_levels[index] {
                MSLevel::MS1 => self.read_single_frame(index),
                _ => Frame::default(),
            })
            .collect()
    }

    fn read_all_ms2_frames(&self) -> Vec<Frame> {
        (0..self.tdf_bin_reader.len())
            .into_par_iter()
            .map(|index| match self.ms_levels[index] {
                MSLevel::MS2 => self.read_single_frame(index),
                _ => Frame::default(),
            })
            .collect()
    }
}
