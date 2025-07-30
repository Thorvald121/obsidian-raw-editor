// src/main.rs
// Dependencies in Cargo.toml:
// eframe = "0.23"
// egui = "0.23"
// image = "0.24"
// rfd = "0.5"
// rawloader = "0.37.1"

use eframe::{egui, run_native, App, Frame, IconData, NativeOptions};
use egui::{ColorImage, TextureOptions, Color32, ComboBox};
use image::{DynamicImage, imageops};
use rawloader::{decode_file, RawImageData};
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

// Worker job for non-blocking adjustments
struct WorkerJob {
    image: DynamicImage,
    exposure: f32,
    contrast: f32,
    saturation: f32,
    vibrance: f32,
}

// Themes
const THEME_NAMES: &[&str] = &[
    "Obsidian Dark",
"Obsidian Light",
"Purple Dark",
"Solarized Light",
];

struct ObsApp {
    // Image data
    current_image: Option<DynamicImage>,
    texture: Option<egui::TextureHandle>,
    history: Vec<DynamicImage>,
    future: Vec<DynamicImage>,
    zoom: f32,
    // Adjustment parameters
    exposure: f32,
    contrast: f32,
    saturation: f32,
    vibrance: f32,
    // Theme index
    theme: usize,
    // Worker channel
    job_sender: Sender<WorkerJob>,
    result_receiver: Receiver<ColorImage>,
}

impl Default for ObsApp {
    fn default() -> Self {
        // create channels
        let (tx_job, rx_job) = channel::<WorkerJob>();
        let (tx_res, rx_res) = channel::<ColorImage>();
        // spawn worker
        thread::spawn(move || {
            while let Ok(job) = rx_job.recv() {
                let mut img = job.image;
                // Exposure
                if job.exposure.abs() > f32::EPSILON {
                    let v = (job.exposure * 100.0) as i32;
                    img = DynamicImage::ImageRgba8(
                        imageops::brighten(&img.to_rgba8(), v)
                    );
                }
                // Contrast
                if job.contrast.abs() > f32::EPSILON {
                    img = DynamicImage::ImageRgba8(
                        imageops::contrast(&img.to_rgba8(), job.contrast * 100.0)
                    );
                }
                // TODO: saturation & vibrance
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let flat = rgba.into_flat_samples();
                let pixels = flat.as_slice();
                let ci = ColorImage::from_rgba_unmultiplied(size, pixels);
                let _ = tx_res.send(ci);
            }
        });
        Self {
            current_image: None,
            texture: None,
            history: Vec::new(),
            future: Vec::new(),
            zoom: 1.0,
            exposure: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            vibrance: 0.0,
            theme: 0,
            job_sender: tx_job,
            result_receiver: rx_res,
        }
    }
}

impl ObsApp {
    // Enqueue a processing job
    fn queue_job(&self) {
        if let Some(img) = &self.current_image {
            let job = WorkerJob {
                image: img.clone(),
                exposure: self.exposure,
                contrast: self.contrast,
                saturation: self.saturation,
                vibrance: self.vibrance,
            };
            let _ = self.job_sender.send(job);
        }
    }
    // History operations
    fn commit(&mut self) {
        if let Some(img) = &self.current_image {
            self.history.push(img.clone());
            self.future.clear();
        }
    }
    fn undo(&mut self) {
        if let Some(prev) = self.history.pop() {
            if let Some(curr) = &self.current_image {
                self.future.push(curr.clone());
            }
            self.current_image = Some(prev);
            self.queue_job();
        }
    }
    fn redo(&mut self) {
        if let Some(next) = self.future.pop() {
            if let Some(curr) = &self.current_image {
                self.history.push(curr.clone());
            }
            self.current_image = Some(next);
            self.queue_job();
        }
    }
    fn reset(&mut self) {
        if let Some(first) = self.history.first().cloned() {
            self.current_image = Some(first);
            self.future.clear();
            self.queue_job();
        }
    }
}

impl App for ObsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // receive processed image
        if let Ok(ci) = self.result_receiver.try_recv() {
            let tex = ctx.load_texture("main_image", ci, TextureOptions::default());
            self.texture = Some(tex);
        }
        // apply theme
        match self.theme {
            0 => ctx.set_visuals(egui::Visuals::dark()),
            1 => ctx.set_visuals(egui::Visuals::light()),
            2 => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = Color32::from_rgb(40,30,80);
                v.faint_bg_color = Color32::from_rgb(50,40,90);
                ctx.set_visuals(v);
            }
            3 => {
                let mut v = egui::Visuals::light();
                v.widgets.hovered.bg_fill = Color32::from_rgb(250,240,210);
                ctx.set_visuals(v);
            }
            _ => {}
        }
        // Top ribbon
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Image or RAW", &["png","jpg","jpeg","tif","tiff"]).pick_file()
                        {
                            // load JPEG/PNG or RAW
                            let dyn_img = if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                match ext.to_lowercase().as_str() {
                                    "tif" | "tiff" => {
                                        // RAW via rawloader
                                        let _raw = decode_file(&path).ok();
                                        // placeholder blank
                                        DynamicImage::new_luma8(1,1)
                                    }
                                    _ => image::open(&path).unwrap(),
                                }
                            } else { image::open(&path).unwrap() };
                            self.current_image = Some(dyn_img.clone());
                            self.history.clear(); self.future.clear();
                            self.commit(); self.queue_job();
                        }
                }
                if ui.button("Undo").clicked() { self.undo(); }
                if ui.button("Redo").clicked() { self.redo(); }
                if ui.button("Reset").clicked() { self.reset(); }
                ComboBox::from_label("Theme").selected_text(THEME_NAMES[self.theme])
                .show_ui(ui, |ui| {
                    for (i,name) in THEME_NAMES.iter().enumerate() {
                        ui.selectable_value(&mut self.theme, i, *name);
                    }
                });
            });
        });
        // Side sliders
        egui::SidePanel::right("side_panel").resizable(false).show(ctx, |ui| {
            ui.heading("Adjustments");
            if self.texture.is_some() {
                let exposure_changed = ui
                .add(egui::Slider::new(&mut self.exposure, -5.0..=5.0).text("Exposure"))
                .changed();
                let contrast_changed = ui
                .add(egui::Slider::new(&mut self.contrast, -100.0..=100.0).text("Contrast"))
                .changed();
                let sat_changed = ui
                .add(egui::Slider::new(&mut self.saturation, -100.0..=100.0).text("Saturation"))
                .changed();
                let vib_changed = ui
                .add(egui::Slider::new(&mut self.vibrance, -100.0..=100.0).text("Vibrance"))
                .changed();
                if exposure_changed || contrast_changed || sat_changed || vib_changed {
                    self.queue_job();
                }
            }
        });
        // Main image
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                egui::ScrollArea::both().show(ui, |ui| {
                    let size = tex.size_vec2() * self.zoom;
                    ui.image(tex.id(), size);
                });
            } else {
                ui.centered_and_justified(|ui| { ui.label("Open an image to get started"); });
            }
        });
    }
}

fn main() {
    // load icon
    let icon = load_icon("icons/icon.png"); // implement load_icon() to read file into IconData

    let native_options = NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1024.0, 768.0)),
        icon_data: Some(icon),
        ..Default::default()
    };
    run_native(
        "Obsidian",
        native_options,
        Box::new(|_| Box::new(ObsApp::default())),
    ).unwrap();
}
