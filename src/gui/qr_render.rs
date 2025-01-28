use csscolorparser::Color;
use egui_extras::RetainedImage;
use image::codecs::png::PngEncoder;
use image::{ColorType, EncodableLayout, ImageBuffer, ImageEncoder, Rgba};
use qrencode::{QrCode, QrResult};

fn build_binary_image(
    content: &str,
    (dr, dg, db, da): (u8, u8, u8, u8),
    (lr, lg, lb, la): (u8, u8, u8, u8),
    quiet_zone: bool,
) -> QrResult<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let img = QrCode::new(content)?
        .render::<Rgba<u8>>()
        .quiet_zone(quiet_zone)
        .dark_color(Rgba([dr, dg, db, da]))
        .light_color(Rgba([lr, lg, lb, la]))
        .build();
    Ok(img)
}

pub fn qr_encode(content: impl Into<String>) -> RetainedImage {
    let dark = "#000000";
    let light = "#FFFFFF";
    // RGB colors
    let dark = dark.parse::<Color>().expect("color").to_linear_rgba_u8();
    let light = light.parse::<Color>().expect("color").to_linear_rgba_u8();
    let string = content.into();
    let image =
        build_binary_image(&*string, dark, light, true).expect("error");
    // let image = build_binary_image(&content, dark, light, !args.no_quiet_zone)?;
    let bytes = image.as_bytes();

    let mut result: Vec<u8> = Default::default();
    let encoder = PngEncoder::new(&mut result);
    encoder.write_image(
        bytes,
        image.width(),
        image.height(),
        ColorType::Rgba8,
    ).expect("write");

    RetainedImage::from_image_bytes("qr", &*result).expect("error")
}