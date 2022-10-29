use eframe::{egui, epi};

/// Immediate mode texture manager that supports at most one texture at the time :)
#[derive(Default)]
pub struct TexMngr {
    loaded_url: String,
    texture_id: Option<egui::TextureId>,
}

impl TexMngr {
    pub(crate) fn texture(
        &mut self,
        frame: &mut epi::Frame<'_>,
        url: &str,
        image: &Image,
    ) -> Option<egui::TextureId> {
        if self.loaded_url != url {
            if let Some(texture_id) = self.texture_id.take() {
                frame.tex_allocator().free(texture_id);
            }

            self.texture_id = Some(
                frame
                    .tex_allocator()
                    .alloc_srgba_premultiplied(image.size, &image.pixels),
            );
            self.loaded_url = url.to_owned();
        }
        self.texture_id
    }
}

pub struct Image {
    pub size: (usize, usize),
    pub pixels: Vec<egui::Color32>,
}

// https://github.com/LedgerHQ/ledger-sample-apps/blob/master/blue-app-samplesign/demo.py
// https://github.com/LedgerHQ/rust-app
//https://github.com/LedgerHQ/ledger-nanos-sdk
impl Image {
    pub fn decode(bytes: &[u8]) -> Option<Image> {
        use image::GenericImageView;
        let image = image::load_from_memory(bytes).ok()?;
        let image_buffer = image.to_rgba8();
        let size = (image.width() as usize, image.height() as usize);
        let pixels = image_buffer.into_vec();
        assert_eq!(size.0 * size.1 * 4, pixels.len());
        let pixels = pixels
            .chunks(4)
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();

        Some(Image { size, pixels })
    }
}
