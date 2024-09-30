use derivative::Derivative;
use eframe::egui;
use eframe::egui::Context;
use egui_extras::RetainedImage;
use image::DynamicImage;
use rqrr::MetaData;
use crate::gui::app_loop::LocalState;
use redgold_gui::common::bounded_text_area;
// use crate::gui::image_capture::{CaptureStream, default_stream};
use crate::gui::qr_render::qr_encode;



#[derive(Derivative)]
#[derivative(Clone)]
pub struct QrState {
    pub show_window: bool,
    // pub capture_stream: Option<CaptureStream>,
    pub last_image: Option<DynamicImage>,
    pub contents: Option<String>,
    pub metadata: Option<MetaData>,
    #[derivative(Clone(clone_with = "clone_option_retained_image"))]
    pub retained_image: Option<RetainedImage>
}

pub fn clone_option_retained_image(opt: &Option<RetainedImage>) -> Option<RetainedImage> {
    None
}


#[derive(Derivative)]
#[derivative(Clone)]
pub struct QrShowState {
    pub show_window: bool,
    #[derivative(Clone(clone_with = "clone_option_retained_image"))]
    pub qr_image: Option<RetainedImage>,
    pub qr_text: Option<String>
}

impl QrShowState {
    pub fn enable(&mut self, content: impl Into<String>) {
        self.show_window = true;
        let string = content.into();
        self.qr_text = Some(string.clone());
        let enc = qr_encode(string);
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


pub fn qr_window(
    ctx: &Context, state: &mut LocalState
) {
    if state.qr_state.show_window {
        state.qr_state.read_tick();
    }

    egui::Window::new("QR Code Camera Parser")
        .open(&mut state.qr_state.show_window)
        .resizable(false)
        .collapsible(false)
        .min_width(500.0)
        .default_width(500.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                if let Some(i) = state.qr_state.retained_image.as_ref() {
                    i.show_scaled(ui, 0.2);
                }
            });
        });
}
pub fn qr_show_window(
    ctx: &Context, state: &mut LocalState
) {
    egui::Window::new("QR Code")
        .open(&mut state.qr_show_state.show_window)
        .resizable(false)
        .default_pos(egui::Pos2::new(0.0, 0.0))
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(0.0, 0.0))
        .collapsible(false)
        .min_width(500.0)
        .default_width(500.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                if let Some(i) = state.qr_show_state.qr_image.as_ref() {
                    i.show_scaled(ui, 1.0);
                }
                if let Some(t) = &mut state.qr_show_state.qr_text.clone() {
                    bounded_text_area(ui, t)
                }
            });
        });
}