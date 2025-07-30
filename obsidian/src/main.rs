// src/main.rs
// Cargo.toml dependencies should include:
// eframe = "0.23"
// egui = "0.23"
// image = "0.24"
// rfd = "0.5"
// rawloader = "0.37.1"

use eframe::{egui, run_native, App, Frame, NativeOptions, IconData};
use egui::{ColorImage, TextureOptions, Color32, ComboBox};
use image::{DynamicImage, imageops, ImageBuffer};
use rawloader::{decode_file, RawImageData};
use rfd::FileDialog;
use std::path::{Path, PathBuf};

// Define available themes
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
}

impl Default for ObsApp {
    fn default() -> Self {
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
        }
    }
}

impl ObsApp {
    /// Rebuilds and uploads the texture based on current adjustments.
    fn upload_texture(&mut self, ctx: &egui::Context) {
        if let Some(img) = &self.current_image {
            let mut processed = img.clone();
            let mut applied = false;
            // Exposure adjustment
            if (self.exposure - 0.0).abs() > f32::EPSILON {
                let val = (self.exposure * 100.0) as i32;
                processed = DynamicImage::ImageRgba8(
                    imageops::brighten(&processed.to_rgba8(), val)
                );
                applied = true;
            }
            // Contrast adjustment
            if (self.contrast - 0.0).abs() > f32::EPSILON {
                processed = DynamicImage::ImageRgba8(
                    imageops::contrast(&processed.to_rgba8(), self.contrast * 100.0)
                );
                applied = true;
            }
            // TODO: implement saturation/vibrance adjustments
            let final_img = if applied { processed } else { img.clone() };

            // Convert to RGBA and upload
            let rgba = final_img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let flat = rgba.into_flat_samples();
            let pixels = flat.as_slice();
            let color_image = ColorImage::from_rgba_unmultiplied(size, pixels);
            let tex = ctx.load_texture("main_image", color_image, TextureOptions::default());
            self.texture = Some(tex);
        }
    }

    fn commit(&mut self) {
        if let Some(img) = &self.current_image {
            self.history.push(img.clone());
            self.future.clear();
        }
    }

    fn undo(&mut self, ctx: &egui::Context) {
        if let Some(prev) = self.history.pop() {
            if let Some(curr) = &self.current_image {
                self.future.push(curr.clone());
            }
            self.current_image = Some(prev);
            self.upload_texture(ctx);
        }
    }

    fn redo(&mut self, ctx: &egui::Context) {
        if let Some(next) = self.future.pop() {
            if let Some(curr) = &self.current_image {
                self.history.push(curr.clone());
            }
            self.current_image = Some(next);
            self.upload_texture(ctx);
        }
    }

    fn reset(&mut self, ctx: &egui::Context) {
        if let Some(first) = self.history.first().cloned() {
            self.current_image = Some(first);
            self.future.clear();
            self.upload_texture(ctx);
        }
    }
}

impl App for ObsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Apply current theme visuals
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

        // Top panel: Open, Undo, Redo, Reset, Theme
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Image", &[
                            "png","jpg","jpeg","tif","tiff",
                            "cr2","nef","arw","dng","raf","rw2","orf",
                        ])
                        .pick_file()
                        {
                            let ext = path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                            let raw_exts = [
                                "cr2","nef","arw","dng","raf","rw2","orf",
                            ];
                            if raw_exts.contains(&ext.as_str()) {
                                if let Ok(raw) = decode_file(&path) {
                                    let w = raw.width as u32;
                                    let h = raw.height as u32;
                                    match raw.data {
                                        RawImageData::Integer(data) => {
                                            let mut rgba = Vec::with_capacity((w*h*4) as usize);
                                            for &v in &data {
                                                let v8 = (v >> 8) as u8;
                                                rgba.extend_from_slice(&[v8, v8, v8, 255]);
                                            }
                                            if let Some(buf) = ImageBuffer::from_raw(w, h, rgba) {
                                                let dynimg = DynamicImage::ImageRgba8(buf);
                                                self.current_image = Some(dynimg.clone());
                                                self.history.clear(); self.future.clear();
                                                self.history.push(dynimg.clone());
                                                self.upload_texture(ctx);
                                            }
                                        }
                                        RawImageData::Float(data) => {
                                            let mut rgba = Vec::with_capacity((w*h*4) as usize);
                                            for &v in &data {
                                                let v8 = (v * 255.0).clamp(0.0, 255.0) as u8;
                                                rgba.extend_from_slice(&[v8, v8, v8, 255]);
                                            }
                                            if let Some(buf) = ImageBuffer::from_raw(w, h, rgba) {
                                                let dynimg = DynamicImage::ImageRgba8(buf);
                                                self.current_image = Some(dynimg.clone());
                                                self.history.clear(); self.future.clear();
                                                self.history.push(dynimg.clone());
                                                self.upload_texture(ctx);
                                            }
                                        }
                                    }
                                }
                            } else if let Ok(img) = image::open(&path) {
                                self.current_image = Some(img.clone());
                                self.history.clear(); self.future.clear();
                                self.history.push(img.clone());
                                self.upload_texture(ctx);
                            }
                        }
                }
                if ui.button("Undo").clicked()  { self.undo(ctx); }
                if ui.button("Redo").clicked()  { self.redo(ctx); }
                if ui.button("Reset").clicked() { self.reset(ctx); }

                ui.separator();
                ComboBox::from_label("Theme")
                .selected_text(THEME_NAMES[self.theme])
                .show_ui(ui, |ui| {
                    for (i, &name) in THEME_NAMES.iter().enumerate() {
                        ui.selectable_value(&mut self.theme, i, name);
                    }
                });
            });
        });

        // Right panel: adjustments
        egui::SidePanel::right("side_panel").show(ctx, |ui| {
            ui.heading("Adjustments");
            if self.texture.is_some() {
                let _ = ui.add(
                    egui::Slider::new(&mut self.zoom, 0.1..=10.0)
                    .text("Zoom").logarithmic(true)
                );
                let resp = ui.add(
                    egui::Slider::new(&mut self.exposure, -1.0..=1.0)
                    .text("Exposure")
                ); if resp.drag_released() { self.upload_texture(ctx); self.commit(); }
                let resp = ui.add(
                    egui::Slider::new(&mut self.contrast, -1.0..=1.0)
                    .text("Contrast")
                ); if resp.drag_released() { self.upload_texture(ctx); self.commit(); }
                let resp = ui.add(
                    egui::Slider::new(&mut self.saturation, -1.0..=1.0)
                    .text("Saturation")
                ); if resp.drag_released() { self.upload_texture(ctx); self.commit(); }
                let resp = ui.add(
                    egui::Slider::new(&mut self.vibrance, -1.0..=1.0)
                    .text("Vibrance")
                ); if resp.drag_released() { self.upload_texture(ctx); self.commit(); }
            }
        });

        // Main area: display & pan/zoom
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                egui::ScrollArea::both().show(ui, |ui| {
                    let size = tex.size_vec2() * self.zoom;
                    ui.image((tex.id(), size));
                });
            } else {
                ui.centered_and_justified(|ui| { ui.label("Open an image to get started"); });
            }
        });
    }
}

fn main() {
    // ── Application Icon ────────────────────────────────────────────────
    // Construct absolute path to the icon in the project-root `icons/` folder
    let icon_data = {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let icon_path = manifest_dir.join("icons").join("icon.png");
        match image::open(&icon_path) {
            Ok(img) => {
                let rgba = img.into_rgba8();
                let (w, h) = rgba.dimensions();
                Some(IconData {
                    width: w,
                    height: h,
                    rgba: rgba.into_raw(),
                })
            }
            Err(err) => {
                eprintln!("Warning: failed to load icon at {}: {}", icon_path.display(), err);
                None
            }
        }
    };

    let native_options = NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1024.0, 768.0)),
        icon_data,
        ..Default::default()
    };

    let _ = run_native(
        "Obsidian (M8)",
                       native_options,
                       Box::new(|_cc| Box::new(ObsApp::default())),
    );
}
