use image::RgbImage;
use redgold_schema::structs::Hash;

/// For each RGB value for each pixel in the image, use a byte from the hash.
/// Therefore, the length of the data must exceed the number of pixels.
fn generate_image(height: u32, width: u32, data: &Vec<u8>) -> image::RgbImage {
    assert!(data.len() as u32 >= height*width);
    let mut img: image::RgbImage = image::ImageBuffer::new(width, height);
    let mut data_iter = data.iter();
    for (_x, _y, pixel) in img.enumerate_pixels_mut() {
        let r = data_iter.next().unwrap();
        let g = data_iter.next().unwrap();
        let b = data_iter.next().unwrap();
        *pixel = image::Rgb([*r, *g, *b]);
    }

    img
}

fn image_from_bytes(hash_vec: Vec<u8>) -> RgbImage {
    let mut all_bytes: Vec<u8> = Vec::new();
    for _ in 0..20 {
        all_bytes.extend(&hash_vec.clone());
    }
    return generate_image(20, 20, &all_bytes)
}

// #[test]
// fn debug_img_gen() {
//     let mut all_bytes: Vec<u8> = Vec::new();
//     let mut h = Hash::from_string("test");
//     for i in 0..20 {
//         all_bytes.extend(h.vec());
//         // h = Hash::calc_bytes(h.vec());
//     }
//     let i1 = all_bytes.len();
//     println!("{:?}", i1);
//     // println!("{:?}", i1);
//     let buffer = generate_image(20, 20, &all_bytes);
//     // buffer.as_raw()
//     buffer.save("test01.png").unwrap();
// }


use svg::node::element;

use rand::Rng;

use noise::{NoiseFn, OpenSimplex};
use noise;

fn noise_color_hsl(noise: noise::OpenSimplex, offset_x: f64, offset_y: f64, x: f64, y: f64, color_multi: f64) -> String {
    let color_h = (noise.get([(offset_x + x as f64) * 0.001, (offset_y + y as f64) * 0.001]) + 1.0) * color_multi;
    let color_s = 50;
    let color_l = 50;

    return format!("hsl({}, {}%, {}%)", color_h, color_s, color_l);
}

fn rand_color_hsl(mut rng: rand::rngs::ThreadRng) -> String {
    let color_h = rng.gen_range(0..358);
    let color_s = rng.gen_range(30..70);
    let color_l = rng.gen_range(30..70);

    return format!("hsl({}, {}%, {}%)", color_h, color_s, color_l);
}

fn polygon(start_x: i32, start_y: i32, size: f64, sides: i32, angle_offset: f64) -> element::path::Data {
    let mut data = element::path::Data::new()
        .move_to((start_x, start_y));

    for i in 0..sides {
        let angle = angle_offset + (std::f64::consts::PI + 2.0 * std::f64::consts::PI * i as f64 / sides as f64);
        let x = size * angle.sin();
        let y = size * angle.cos();

        data = data.line_by((x, y));
    }

    return data;
}

#[test]
fn debug_rart() {
    let doc_size_x = 1920;
    let doc_size_y = 1080;

    let mut rng = rand::thread_rng();
    let noise = OpenSimplex::new();

    let mut paths = Vec::new();

    let offset_x = rng.gen_range(0.0..1000000.0);
    let offset_y = rng.gen_range(0.0..1000000.0);

    let back_size = rng.gen_range(5..50);
    let back_sides = rng.gen_range(3..10);

    let max_coords = doc_size_x + doc_size_y * doc_size_y / back_size;

    let color_multi = rng.gen_range(20.0..160.0);

    for y in (0..doc_size_y).step_by(back_size) {
        for x in (0..doc_size_x).step_by(back_size) {
            print!("\rDrawing background {:.2}% ({}/{})", (x as f64 + y as f64 * doc_size_y as f64 / back_size as f64) / max_coords as f64 * 100.0, x + y * doc_size_y / back_size, max_coords);

            //let data = polygon(x, y, 5.0 + 1.0, 4);
            let data = polygon(x as i32, y as i32, back_size as f64 + 1.0, back_sides, rng.gen_range(-360.0..360.0));

            let path = element::Path::new()
                .set("fill", noise_color_hsl(noise, offset_x, offset_y, x as f64, y as f64, color_multi))
                .set("stroke", "none")
                .set("stroke-width", 0)
                .set("d", data);

            paths.push(path);
        }
    }

    print!("\rDrawing background {}% ({}/{})\n", 100, max_coords, max_coords);

    let polygons = rng.gen_range(0..100);

    for i in 0..polygons {
        print!("\rPlacing polygons {:.2}% ({}/{})", (i + 1) / polygons * 100, i + 1, polygons);

        let start_x = rng.gen_range(10..(doc_size_x - 10)) as i32;
        let start_y = rng.gen_range(10..(doc_size_y - 10)) as i32;

        let size = rng.gen_range(1.0..50.0);

        let sides = rng.gen_range(3..20);

        let data = polygon(start_x, start_y, size, sides, 0.0);

        let path = element::Path::new()
            .set("fill", rand_color_hsl(rng.clone()))
            .set("stroke", "black")
            .set("stroke-width", 3)
            .set("d", data);

        paths.push(path);
    }

    print!("\rPlacing polygons {}% ({}/{})\n", 100, polygons, polygons);

    print!("Generating document (Step 1/3)");

    let mut document = svg::Document::new()
        .set("viewBox", (0, 0, 1920, 1080));

    print!("\rGenerating document (Step 2/3)");

    for p in paths {
        document = document.add(p);
    }

    print!("\rGenerating document (Step 3/3)");

    //svg::save("out.svg", &document).unwrap();
    //
    // let save_string: String = rng.sample_iter(&rand::distributions::Alphanumeric)
    //     .take(8)
    //     .collect();

    svg::save("debug.svg", &document).expect("");

    // return save_string;
}