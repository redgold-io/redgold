use csscolorparser::Color;
use eframe::egui::load::Bytes;
use eframe::egui::Image;
use image::codecs::png::PngEncoder;
use image::{ColorType, EncodableLayout, ImageBuffer, ImageEncoder, Rgba};
use qrencode::{QrCode, QrResult};

fn build_qr_image_internal(
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
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn string_hash(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

pub fn qr_encode_image(content: impl Into<String>) -> Image<'static> {


    let dark = "#000000";
    let light = "#FFFFFF";
    // RGB colors
    let dark = dark.parse::<Color>().expect("color").to_linear_rgba_u8();
    let light = light.parse::<Color>().expect("color").to_linear_rgba_u8();
    let string = content.into();

    let hash = string_hash(&string);
    let string1 = string.clone();
    let image =
        build_qr_image_internal(&*string1, dark, light, true).expect("error");
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

    // Convert Vec<u8> into Bytes
    let bytes: Bytes = result.into();


    Image::from_bytes(format!("bytes://qr/{}", hash) , bytes)
}