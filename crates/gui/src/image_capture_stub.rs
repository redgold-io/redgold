use crate::functionality::capture::CaptureLike;
use image::DynamicImage;
use redgold_schema::{error_info, RgResult};
use rqrr::MetaData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStream {
    pub active_device: Option<String>
}

impl CaptureLike for CaptureStream {
    fn read_qr(&mut self) -> RgResult<(DynamicImage, RgResult<(MetaData, String)>)> {
        Err(error_info("Not implemented"))
    }
    fn get_device_names(&self) -> RgResult<Vec<String>> {
        Err(error_info("Not implemented"))
    }
    fn change(&mut self, device_name: String) -> RgResult<()> {
        Err(error_info("Not implemented"))
    }
    fn new(device_name: Option<String>) -> RgResult<Self> {
        Err(error_info("Not implemented"))
    }
}