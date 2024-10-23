use crate::functionality::capture::CaptureLike;
use crate::image_capture_openpnp::{convert_to_image, get_devices, get_stream, qr_parse_capture, read_stream};
use image::DynamicImage;
use openpnp_capture::Stream;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::{RgResult, SafeOption};
use rqrr::MetaData;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, Debug)]
pub struct CaptureStream {
    #[serde(skip)]
    pub stream: Option<Arc<Mutex<Stream>>>,
}

impl CaptureLike for CaptureStream {
    fn read_qr(&mut self) -> RgResult<(DynamicImage, RgResult<(MetaData, String)>)> {
        if let Some(s) = &self.stream {
            let mut stream_lock = s.lock().unwrap();
            let last_bytes = read_stream(&mut stream_lock)?;
            let image = convert_to_image(last_bytes.clone(), &stream_lock.format())?;
            let qr = qr_parse_capture(&image);
            Ok((image, qr))
        } else {
            "No stream".to_error()
        }
    }
    fn get_device_names(&self) -> RgResult<Vec<String>> {
        let devices = get_devices()?;
        let mut res = vec![];
        for d in devices {
            res.push(d.name.clone())
        }
        Ok(res)
    }
    fn change(&mut self, device_name: String) -> RgResult<()> {
        let new = Self::new(Some(device_name))?;
        self.stream = new.stream;
    }
    fn new(device_name: Option<String>) -> RgResult<Self> {
        let devices = get_devices()?;
        let dev = match device_name {
            None => {
                devices.get(0).ok_msg("No device")?
            }
            Some(n) => {
                devices.iter().find(|d| d.name == n).ok_msg("No device")?
            }
        };
        let formats = dev.formats();
        let f = formats.get(0).ok_msg("No format")?;
        let s = get_stream(dev, f)?;
        let s = CaptureStream {
            stream: Some(Arc::new(Mutex::new(s))),
        };
        Ok(s)
    }
}
