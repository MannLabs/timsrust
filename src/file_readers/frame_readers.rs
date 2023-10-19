use crate::{Frame, FrameType};

use self::tdf_reader::TDFReader;

use super::file_formats::FileFormat;

pub mod tdf_reader;

pub trait ReadableFrames {
    fn read_single_frame(&self, index: usize) -> Frame;

    fn read_all_frames(&self) -> Vec<Frame>;

    fn read_all_ms1_frames(&self) -> Vec<Frame> {
        // I am assuming this can be over-written if there is a more
        // performant way to do this for a specific file format.
        let frames: Vec<Frame> = self.read_all_frames();
        let ms1_frames: Vec<Frame> = frames
            .into_iter()
            .filter(|frame| frame.frame_type == FrameType::MS1)
            .collect();
        ms1_frames
    }

    fn read_all_ms2_frames(&self) -> Vec<Frame> {
        // I am assuming this can be over-written if there is a more
        // performant way to do this for a specific file format.
        let frames: Vec<Frame> = self.read_all_frames();
        let dia_frames: Vec<Frame> = frames
            .into_iter()
            .filter(|frame| frame.frame_type == FrameType::MS2(AcquisitionType::DDAPASEF))
            .collect();
        dia_frames
    }

    fn read_all_dia_frames(&self) -> Vec<Frame> {
        // I am assuming this can be over-written if there is a more
        // performant way to do this for a specific file format.
        let frames: Vec<Frame> = self.read_all_frames();
        let dia_frames: Vec<Frame> = frames
            .into_iter()
            .filter(|frame| frame.frame_type == FrameType::MS2(AcquisitionType::DIAPASEF))
            .collect();
        dia_frames
    }
}

impl FileFormat {
    fn unwrap_frame_reader(&self) -> Box<dyn ReadableFrames> {
        let result = match &self {
            Self::DFolder(path) => Box::new(TDFReader::new(
                &path.to_str().unwrap_or_default().to_string(),
            )) as Box<dyn ReadableFrames>,
            Self::MS2Folder(path) => panic!(
                "Folder {:} is not frame readable",
                path.to_str().unwrap_or_default().to_string()
            ),
        };
        result
    }
}

impl ReadableFrames for FileFormat {
    fn read_single_frame(&self, index: usize) -> Frame {
        self.unwrap_frame_reader().read_single_frame(index)
    }

    fn read_all_frames(&self) -> Vec<Frame> {
        self.unwrap_frame_reader().read_all_frames()
    }

    fn read_all_ms1_frames(&self) -> Vec<Frame> {
        self.unwrap_frame_reader().read_all_ms1_frames()
    }

    fn read_all_ms2_frames(&self) -> Vec<Frame> {
        self.unwrap_frame_reader().read_all_ms2_frames()
    }
    
    fn read_all_dia_frames(&self) -> Vec<Frame> {
        self.unwrap_frame_reader().read_all_dia_frames()
    }
}
