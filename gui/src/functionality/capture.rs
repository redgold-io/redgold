use image::DynamicImage;
use rqrr::MetaData;
use redgold_schema::RgResult;

pub trait CaptureLike {
    fn read_qr(&mut self) -> RgResult<(DynamicImage, RgResult<(MetaData, String)>)>;
    fn get_device_names(&self) -> RgResult<Vec<String>>;
    fn change(&mut self, device_name: String) -> RgResult<()>;
    fn new(device_name: Option<String>) -> RgResult<Self>;
}
