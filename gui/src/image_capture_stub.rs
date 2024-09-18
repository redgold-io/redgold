
#[derive(Debug)]

pub struct CaptureStream {

}

impl CaptureStream {
    pub fn read_qr(&mut self) -> RgResult<(DynamicImage, RgResult<(MetaData, String)>)> {
        Err(error_info("Not implemented"))
    }
}