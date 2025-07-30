// src/main.rs
// Dependencies in Cargo.toml:
// eframe = "0.23"
// egui = "0.23"
// image = "0.24"
// rfd = "0.5"
// rawloader = "0.37.1"

use eframe::{egui, run_native, App, Frame, NativeOptions};
use egui::{ColorImage, TextureOptions, Color32, ComboBox};
use image::{DynamicImage, imageops, ImageBuffer, Rgba};
use rawloader::decode_file;
use rfd::FileDialog;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Instant, Duration};
use std::thread;

/// Adjust saturation by a given factor (1.0 = no change)
fn saturate(buf: &ImageBuffer<Rgba<u8>, Vec<u8>>, factor: f32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (w, h) = (buf.width(), buf.height());
    let mut out = ImageBuffer::new(w, h);
    for (x, y, pixel) in buf.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let lum = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
        let nr = (lum + (r as f32 - lum) * factor).clamp(0.0, 255.0) as u8;
        let ng = (lum + (g as f32 - lum) * factor).clamp(0.0, 255.0) as u8;
        let nb = (lum + (b as f32 - lum) * factor).clamp(0.0, 255.0) as u8;
        out.put_pixel(x, y, Rgba([nr, ng, nb, a]));
    }
    out
}

// Worker job for non-blocking adjustments
struct WorkerJob {
    image: DynamicImage,
    exposure: f32,
    contrast: f32,
    saturation: f32,
    vibrance: f32,
}

// Theme names
const THEME_NAMES: &[&str] = &[
    "Obsidian Dark",
"Obsidian Light",
"Purple Dark",
"Solarized Light",
];

struct ObsApp {
    current_image: Option<DynamicImage>,
    texture: Option<egui::TextureHandle>,
    history: Vec<DynamicImage>,
    future: Vec<DynamicImage>,
    zoom: f32,
    exposure: f32,
    contrast: f32,
    saturation: f32,
    vibrance: f32,
    theme: usize,
    job_sender: Sender<WorkerJob>,
    result_receiver: Receiver<ColorImage>,
    last_job: Instant,
    debounce: Duration,
}

impl Default for ObsApp {
    fn default() -> Self {
        let (tx_job, rx_job) = channel::<WorkerJob>();
        let (tx_res, rx_res) = channel::<ColorImage>();
        thread::spawn(move || {
            while let Ok(job) = rx_job.recv() {
                let mut img = job.image;
                // Exposure
                if job.exposure.abs() > f32::EPSILON {
                    img = DynamicImage::ImageRgba8(
                        imageops::brighten(&img.to_rgba8(), (job.exposure * 100.0) as i32),
                    );
                }
                // Contrast
                if job.contrast.abs() > f32::EPSILON {
                    img = DynamicImage::ImageRgba8(
                        imageops::contrast(&img.to_rgba8(), job.contrast * 100.0),
                    );
                }
                // Saturation
                if job.saturation.abs() > f32::EPSILON {
                    img = DynamicImage::ImageRgba8(
                        saturate(&img.to_rgba8(), 1.0 + job.saturation / 100.0),
                    );
                }
                // Vibrance
                if job.vibrance.abs() > f32::EPSILON {
                    img = DynamicImage::ImageRgba8(
                        saturate(&img.to_rgba8(), 1.0 + job.vibrance / 100.0),
                    );
                }
                // Convert to ColorImage for rendering
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
            last_job: Instant::now() - Duration::from_millis(100),
            debounce: Duration::from_millis(100),
        }
    }
}

impl ObsApp {
    fn queue_job(&mut self) {
        if let Some(img) = &self.current_image {
            let now = Instant::now();
            if now.duration_since(self.last_job) >= self.debounce {
                self.last_job = now;
                let _ = self.job_sender.send(WorkerJob {
                    image: img.clone(),
                                             exposure: self.exposure,
                                             contrast: self.contrast,
                                             saturation: self.saturation,
                                             vibrance: self.vibrance,
                });
            }
        }
    }

    fn commit(&mut self) {(&mut self) {
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
        // Handle zoom via Cmd + scroll
        let scroll = ctx.input(|i| i.scroll_delta);
        let mods = ctx.input(|i| i.modifiers);
        if mods.command && scroll.y != 0.0 {
            let factor = 1.0 + scroll.y * 0.01;
            self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
        }

        // Receive rendering results
        if let Ok(ci) = self.result_receiver.try_recv() {
            let tex = ctx.load_texture("main_image", ci, TextureOptions::default());
            self.texture = Some(tex);
        }

        // Apply theme
        match self.theme {
            0 => ctx.set_visuals(egui::Visuals::dark()),
            1 => ctx.set_visuals(egui::Visuals::light()),
            2 => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = Color32::from_rgb(40, 30, 80);
                v.faint_bg_color = Color32::from_rgb(50, 40, 90);
                ctx.set_visuals(v);
            }
            3 => {
                let mut v = egui::Visuals::light();
                v.widgets.hovered.bg_fill = Color32::from_rgb(250, 240, 210);
                ctx.set_visuals(v);
            }
            _ => {}
        }

        // Top ribbon
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Image or RAW", &["png", "jpg", "jpeg", "tif", "tiff"] )
                        .pick_file()
                        {
                            let ext = path.extension()
    .and_then(|e| e.to_str())
    .map(|s| s.to_lowercase());
    let dyn_img = match ext.as_deref() {
        Some("tif") | Some("tiff") => {
            let _ = decode_file(&path);
            DynamicImage::new_luma8(1, 1)
        }
        _ => image::open(&path).unwrap(),
    };
                            self.current_image = Some(dyn_img.clone());
                            self.history.clear();
                            self.future.clear();
                            self.commit();
                            self.queue_job();
                        }
                }
                if ui.button("Undo").clicked() { self.undo(); }
                if ui.button("Redo").clicked() { self.redo(); }
                if ui.button("Reset").clicked() { self.reset(); }
                ComboBox::from_label("Theme")
                .selected_text(THEME_NAMES[self.theme])
                .show_ui(ui, |ui| {
                    for (i, &name) in THEME_NAMES.iter().enumerate() {
                        ui.selectable_value(&mut self.theme, i, name);
                    }
                });
            });
        });

        // Side panel for adjustments
        egui::SidePanel::right("side_panel")
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Adjustments");
            let mut changed = false;
            changed |= ui.add(egui::Slider::new(&mut self.exposure, -100.0..=100.0).text("Exposure")).changed();
            changed |= ui.add(egui::Slider::new(&mut self.contrast, -100.0..=100.0).text("Contrast")).changed();
            changed |= ui.add(egui::Slider::new(&mut self.saturation, -100.0..=100.0).text("Saturation")).changed();
            changed |= ui.add(egui::Slider::new(&mut self.vibrance, -100.0..=100.0).text("Vibrance")).changed();
            if changed {
                self.queue_job();
            }
        });

        // Main image area
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                egui::ScrollArea::both().show(ui, |ui| {
                    let size = tex.size_vec2() * self.zoom;
                    ui.image((tex.id(), size));
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Open an image to get started");
                });
            }
        });
    }
}

fn main() {
    let native_options = NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1024.0, 768.0)),
        ..Default::default()
    };
    run_native(
        "Obsidian",
        native_options,
        Box::new(|_cc| Box::new(ObsApp::default())),
    )
    .unwrap();
}
