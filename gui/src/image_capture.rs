use image::DynamicImage;
use openpnp_capture::Stream;
use rqrr::MetaData;
use redgold_schema::RgResult;
use crate::image_capture_openpnp::{convert_to_image, qr_parse_capture, read_stream};

#[derive(Debug)]
pub struct CaptureStream {
    pub stream: Stream,
}

impl CaptureStream {
    pub fn read_qr(&mut self) -> RgResult<(DynamicImage, RgResult<(MetaData, String)>)> {
        let last_bytes = read_stream(&mut self.stream)?;
        let image = convert_to_image(last_bytes.clone(), &self.stream.format())?;
        let qr = qr_parse_capture(&image);
        Ok((image, qr))
    }
}
