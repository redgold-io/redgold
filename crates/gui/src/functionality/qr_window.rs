// use crate::gui::image_capture::{CaptureStream, default_stream};
use eframe::egui;
use eframe::egui::{Context, Image};
use egui_extras::RetainedImage;
use image::DynamicImage;
use redgold_common::external_resources::ExternalNetworkResources;
use crate::common::bounded_text_area;
use rqrr::MetaData;
use crate::functionality::qr_render::qr_encode_image;

pub fn clone_option_retained_image(opt: &Option<RetainedImage>) -> Option<RetainedImage> {
    None
}



// #[derive(Derivative)]
// #[derivative(Clone)]
pub struct QrState {
    pub show_window: bool,
    // pub capture_stream: Option<CaptureStream>,
    pub last_image: Option<DynamicImage>,
    pub contents: Option<String>,
    pub metadata: Option<MetaData>,
    // #[derivative(Clone(clone_with = "clone_option_retained_image"))]
    // #[derivative(Clone(bound=""))]
    pub retained_image: Option<Image<'static>>
}

impl Clone for QrState {
    fn clone(&self) -> Self {
        Self {
            show_window: self.show_window,
            last_image: self.last_image.clone(),
            contents: self.contents.clone(),
            metadata: self.metadata.clone(),
            retained_image: None,
        }
    }

}

impl Clone for QrShowState {
    fn clone(&self) -> Self {
        Self {
            show_window: self.show_window,
            qr_image: None,
            qr_text: self.qr_text.clone(),
        }
    }
}

// #[derive(Derivative)]
// #[derivative(Clone)]
pub struct QrShowState {
    pub show_window: bool,
    // #[derivative(Clone(clone_with = "clone_option_retained_image"))]
    pub qr_image: Option<Image<'static>>,
    pub qr_text: Option<String>
}

impl QrShowState {
    pub fn enable(&mut self, content: impl Into<String>) {
        self.show_window = true;
        let string = content.into();
        self.qr_text = Some(string.clone());
        let enc = qr_encode_image(string);
        self.qr_image = Some(enc);
    }
}

impl Default for QrShowState {
    fn default() -> Self {
        Self {
            show_window: false,
            qr_image: None,
            qr_text: None,
        }
    }
}

impl Default for QrState {
    fn default() -> Self {
        Self {
            show_window: false,
            // capture_stream: None,
            last_image: None,
            contents: None,
            metadata: None,
            retained_image: None,
        }
    }
}

impl QrState {
    pub fn enable(&mut self) {
        self.show_window = true;
        // self.capture_stream = Some(default_stream().expect("error"));
        self.contents = None;
        self.metadata = None;
        self.retained_image = None;
        self.last_image = None;
    }
    pub fn disable(&mut self) {
        self.show_window = false;
        // self.capture_stream = None;
    }

    pub fn read_tick(&mut self) {
        // if let Some(c) = &mut self.capture_stream {
        //     let (image, read) = c.read_qr().expect("Failed");
        //     let ci = egui::ColorImage::from_rgb([image.width() as usize, image.height() as usize],
        //                                image.to_rgb8().as_raw()
        //     );
        //     if let Ok((md, s)) = read {
        //         self.contents = Some(s);
        //         self.metadata = Some(md);
        //         // TODO: Checkbox for keeping screen on capture?
        //         self.disable()
        //     }
        //     let retained = RetainedImage::from_color_image("camera_frame", ci);
        //     self.retained_image = Some(retained);
        //     self.last_image = Some(image);
        // }
    }

}



impl QrState {
    pub fn qr_window<E>(&mut self, ctx: &Context)
    where
        E: ExternalNetworkResources + Clone + Send + Sync + 'static
    {
        if self.show_window {
            self.read_tick();
        }
        let mut show = self.show_window;
        egui::Window::new("QR Code Camera Parser")
            .open(&mut show)
            .resizable(false)
            .collapsible(false)
            .min_width(500.0)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    if let Some(i) = self.retained_image.as_ref() {
                        // i.show_scaled(ui, 0.2);
                    }
                });
            });
        self.show_window = show;
    }
}
impl QrShowState {
pub fn qr_show_window<E>(
        &mut self,
        ctx: &Context
    ) where
        E: ExternalNetworkResources + Clone + Send + Sync + 'static
    {

        let mut show = self.show_window;
        egui::Window::new("QR Code")
            .open(&mut show)
            .resizable(false)
            .default_pos(egui::Pos2::new(0.0, 0.0))
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(0.0, 0.0))
            .collapsible(false)
            .min_width(500.0)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    if let Some(i) = self.qr_image.as_ref() {
                        // i.show_scaled(ui, 1.0);
                    }
                    if let Some(t) = &mut self.qr_text.clone() {
                        bounded_text_area(ui, t);
                    }
                });
            });

        self.show_window = show;
    }
}