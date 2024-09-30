use std::io::Cursor;
use std::thread::sleep;
use std::time::Duration;
use image::{DynamicImage, ImageBuffer};
use openpnp_capture::{Device, Format, Stream};
use rqrr::MetaData;
use redgold_schema::{error_info, ErrorInfoContext, RgResult};
use crate::image_capture::CaptureStream;
use image::DynamicImage::ImageRgb8;
// use nokhwa::Camera;
// use nokhwa::pixel_format::RgbFormat;
// use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

//
// pub fn debug_capture() {
// //     // let image = capture_image().unwrap();
// //     // let mut buf = Cursor::new(Vec::new());
// //     // image.write_to(&mut buf, image::ImageOutputFormat::Png).unwrap();
// //     // let contents = buf.get_mut().to_vec();
// //     // std::fs::write("test.png", contents).unwrap();
// //     // first camera in system
// //     println!("Attempting capture");
// //     let index = CameraIndex::Index(0);
// // // request the absolute highest resolution CameraFormat that can be decoded to RGB.
// //     let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
// // // make the camera
// //     let mut camera = Camera::new(index, requested).unwrap();
// //     println!("Made camera");
// //
// // // get a frame
// //     let frame = camera.frame().unwrap();
// //     println!("Captured Single Frame of {}", frame.buffer().len());
// // // decode into an ImageBuffer
// // //     let decoded = frame.decode_image::<RgbFormat>().unwrap();
// // //     println!("Decoded Frame of {}", decoded.len());
// //
// // use std::sync::atomic::{AtomicUsize, Ordering};
// //     use std::sync::Arc;
// //     use std::time::Duration;
// //
// // // Get a libuvc context
// //     let ctx = uvc::Context::new().expect("Could not get context");
// //
// // // Get a default device
// //     let dev = ctx
// //         .find_device(None, None, None)
// //         .expect("Could not find device");
// //
// // // Or create an iterator over all available devices
// //     let mut _devices = ctx.devices().expect("Could not enumerate devices");
// //
// // // The device must be opened to create a handle to the device
// //     let devh = dev.open().expect("Could not open device");
// //
// // // Most webcams support this format
// //     let format = uvc::StreamFormat {
// //         width: 640,
// //         height: 480,
// //         fps: 30,
// //         format: uvc::FrameFormat::YUYV,
// //     };
// //
// // // Get the necessary stream information
// //     let mut streamh = devh
// //         .get_stream_handle_with_format(format)
// //         .expect("Could not open a stream with this format");
// //
// // // This is a counter, increasing by one for every frame
// // // This data must be 'static + Send + Sync to be used in
// // // the callback used in the stream
// //     let counter = Arc::new(AtomicUsize::new(0));
// //
// // // Get a stream, calling the closure as callback for every frame
// //     let stream = streamh
// //         .start_stream(
// //             |_frame, count| {
// //                 count.fetch_add(1, Ordering::SeqCst);
// //             },
// //             counter.clone(),
// //         ).expect("Could not start stream");
// //
// // // Wait 10 seconds
// //     std::thread::sleep(Duration::new(10, 0));
// //
// // // Explicitly stop the stream
// // // The stream would also be stopped
// // // when going out of scope (dropped)
// //     stream.stop();
// //     println!("Counter: {}", counter.load(Ordering::SeqCst));
//
//
//
// }
//
// pub fn eyes() -> Result<()> {
//     use eye_hal::PlatformContext;
//     use eye_hal::traits::{Context, Device, Stream};
//
//     // Create a context
//     let ctx = PlatformContext::default();
//
//     // Query for available devices.
//     let devices = ctx.query_devices()?;
//
//     // First, we need a capture device to read images from. For this example, let's just choose
//     // whatever device is first in the list.
//     let dev = ctx.open_device(&devices[0])?;
//
//     // Query for available streams and just choose the first one.
//     let streams = dev.query_streams()?;
//     let stream_desc = streams[0].clone();
//     println!("Stream: {:?}", stream_desc);
//
//     // Since we want to capture images, we need to access the native image stream of the device.
//     // The backend will internally select a suitable implementation for the platform stream. On
//     // Linux for example, most devices support memory-mapped buffers.
//     let mut stream = dev.start_stream(&stream_desc)?;
//
//     // Here we create a loop and just capture images as long as the device produces them. Normally,
//     // this loop will run forever unless we unplug the camera or exit the program.
//     loop {
//         let frame = stream
//             .next()
//             .expect("Stream is dead")
//             .expect("Failed to capture frame");
//     }
// }

pub fn test_pnp() {
    use openpnp_capture::{Device, Format, Stream};

    // Fetch some generic device information
    let devices = Device::enumerate();
    println!("Found {} devices.", devices.len());

    for dv in devices.clone() {
        println!("Device: {:?}", dv);
        let d = Device::new(dv).expect("Failed to open device");
        println!("Device: {:?} formats: {:?}", d, d.formats());
    }

    // Choose the first device we see
    let dev = Device::new(devices[0]).expect("Failed to open device");

    let vec1 = dev.formats();
    let f = vec1.get(0).expect("format");


    // Create the stream
    // let format = Format::default().width(1920).height(1080).fps(30);
    let mut stream = Stream::new(&dev, &f).expect("Failed to create stream");

    // Print some format information
    println!(
        "[0] {} ({}x{}@{})",
        stream.format().fourcc,
        stream.format().width,
        stream.format().height,
        stream.format().fps
    );
    //
    // // Prepare a buffer to hold camera frames
    // let mut rgb_buffer = Vec::new();
    //
    // // Capture some frames
    // stream.advance();
    // stream.read(&mut rgb_buffer);

    // Allow the camera to adjust
    sleep(Duration::from_secs(2));

    // Capture a few frames, using the last one
    let mut rgb_buffer = vec![0u8; f.width as usize * f.height as usize * 3]; // Assuming RGB24
    for _ in 0..10 {
        stream.advance();
        stream.read(&mut rgb_buffer).expect("Failed to read frame");
        sleep(Duration::from_millis(100));
    }

    let image = ImageBuffer::from_raw(f.width, f.height, rgb_buffer).unwrap();
    let image = DynamicImage::ImageRgb8(image);
    let mut buf = Cursor::new(Vec::new());
    image.write_to(&mut buf, image::ImageOutputFormat::Png).unwrap();
    let contents = buf.get_mut().to_vec();
    std::fs::write("test.png", contents).unwrap();

}

pub fn default_stream(i: i64) -> RgResult<CaptureStream> {
    let devices = get_devices()?;
    let d = devices.get(i as usize).expect("device 0");
    let formats = d.formats();
    let f = formats.get(0).expect("format 0");
    let mut s = get_stream(d, f)?;
    Ok(CaptureStream{ stream: s })
}

pub fn test_pnp2(option: Option<i64>) -> RgResult<()> {

    let mut s = default_stream(option.unwrap_or(0))?;
    sleep(Duration::from_secs(2));
    let mut last_bytes = vec![];
    let mut iter = 0;
    loop {
        last_bytes = read_stream(&mut s.stream)?;
        let image = convert_to_image(last_bytes.clone(), &s.stream.format())?;
        match qr_parse_capture(&image) {
            Ok((md, s)) => {
                println!("Found QR metadata: {:?}", md);
                println!("Found QR: {}", s);
                save_image(&image);
                break
            }
            Err(e) => {}
        }
        sleep(Duration::from_millis(100));
        iter += 1;
        if iter % 20 == 0 {
            save_image(&image);
        }
    }
    Ok(())

}

pub fn debug_capture(option: Option<i64>) {
    test_pnp2(option).expect("works");
}

#[test]
pub fn test_cap() {

    // debug_capture()

}


pub fn get_devices() -> RgResult<Vec<Device>> {
    let devices = Device::enumerate();
    let mut res = vec![];
    for dv in devices {
        let d = Device::new(dv).ok_or(error_info("Failed to open device"))?;
        res.push(d)
    }
    Ok(res)
}

pub fn get_stream(dev: &Device, format: &Format) -> RgResult<Stream> {
    Stream::new(&dev, &format).ok_or(error_info("Failed to create stream"))
}

pub fn read_stream(stream: &mut Stream) -> RgResult<Vec<u8>> {
    let mut rgb_buffer = vec![0u8; stream.format().width as usize * stream.format().height as usize * 3]; // Assuming RGB24
    stream.advance();
    stream.read(&mut rgb_buffer).error_info("Failed to read frame")?;
    Ok(rgb_buffer)
}

pub fn convert_to_image(rgb_buffer: Vec<u8>, f: &Format) -> RgResult<DynamicImage> {
    let image = ImageBuffer::from_raw(
        f.width, f.height, rgb_buffer).ok_or(error_info("failed to convert image"))?;
    let image = DynamicImage::ImageRgb8(image);
    Ok(image)
}

pub fn save_image(image: &DynamicImage) {
    let mut buf = Cursor::new(Vec::new());
    image.write_to(&mut buf, image::ImageOutputFormat::Png).unwrap();
    let contents = buf.get_mut().to_vec();
    std::fs::write("test.png", contents).unwrap();
}

pub fn qr_parse_capture(image: &DynamicImage) -> RgResult<(MetaData, String)> {
    let image = image.to_luma8();
    let mut img = rqrr::PreparedImage::prepare(image);
    let grids = img.detect_grids();

    if let Some(grid) = grids.first() {
        let (meta, content) = grid.decode().error_info("Bad grid decode")?;
        return Ok((meta, content));
    }
    return Err(error_info("No grid found"));
}
