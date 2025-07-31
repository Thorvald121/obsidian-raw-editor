// src/main.rs
use eframe::{egui, run_native, App, Frame, NativeOptions};
use egui::{ColorImage, TextureOptions, Color32, ComboBox};
use image::DynamicImage;
use rfd::FileDialog;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Instant, Duration};
use std::thread;

mod raw_loader;
mod adjustment_state;
mod history_manager;
mod image_processor;
mod ui_manager;


use raw_loader::{RawLoader, LoadError};
use adjustment_state::AdjustmentState;
use history_manager::HistoryManager;
use image_processor::{ImageProcessor, ProcessingJob, ProcessingResult};

// Theme names (keeping from your original code)
const THEME_NAMES: &[&str] = &[
    "Obsidian Dark",
    "Obsidian Light", 
    "Purple Dark",
    "Solarized Light",
];

pub struct ObsApp {
    // Core components
    raw_loader: RawLoader,
    adjustment_state: AdjustmentState,
    history_manager: HistoryManager,
    
    // Current state
    current_image: Option<DynamicImage>,
    texture: Option<egui::TextureHandle>,
    zoom: f32,
    theme: usize,
    
    // Processing
    job_sender: Sender<ProcessingJob>,
    result_receiver: Receiver<ProcessingResult>,
    last_job: Instant,
    debounce: Duration,
}

impl Default for ObsApp {
    fn default() -> Self {
        let (tx_job, rx_job) = channel::<ProcessingJob>();
        let (tx_res, rx_res) = channel::<ProcessingResult>();
        
        // Spawn worker thread for image processing
        thread::spawn(move || {
            let processor = ImageProcessor::new();
            while let Ok(job) = rx_job.recv() {
                let result = processor.process_image(job);
                let _ = tx_res.send(result);
            }
        });
        
        Self {
            raw_loader: RawLoader::new(),
            adjustment_state: AdjustmentState::default(),
            history_manager: HistoryManager::new(),
            current_image: None,
            texture: None,
            zoom: 1.0,
            theme: 0,
            job_sender: tx_job,
            result_receiver: rx_res,
            last_job: Instant::now() - Duration::from_millis(100),
            debounce: Duration::from_millis(100),
        }
    }
}

impl ObsApp {
    fn load_image(&mut self, path: PathBuf) {
        match self.raw_loader.load_image(&path) {
            Ok(image) => {
                self.current_image = Some(image.clone());
                self.history_manager.clear();
                self.history_manager.push_state(image);
                self.adjustment_state.reset();
                self.queue_processing_job();
                println!("Successfully loaded: {}", path.display());
            }
            Err(e) => {
                eprintln!("Failed to load image {}: {}", path.display(), e);
            }
        }
    }
    
    fn queue_processing_job(&mut self) {
        if let Some(img) = &self.current_image {
            let now = Instant::now();
            if now.duration_since(self.last_job) >= self.debounce {
                self.last_job = now;
                let job = ProcessingJob {
                    image: img.clone(),
                    adjustments: self.adjustment_state.clone(),
                };
                let _ = self.job_sender.send(job);
            }
        }
    }
    
    fn handle_undo(&mut self) {
        if let Some(image) = self.history_manager.undo() {
            self.current_image = Some(image);
            self.adjustment_state.reset();
            self.queue_processing_job();
        }
    }
    
    fn handle_redo(&mut self) {
        if let Some(image) = self.history_manager.redo() {
            self.current_image = Some(image);
            self.adjustment_state.reset();
            self.queue_processing_job();
        }
    }
    
    fn handle_reset(&mut self) {
        if let Some(original) = self.history_manager.get_original() {
            self.current_image = Some(original);
            self.adjustment_state.reset();
            self.queue_processing_job();
        }
    }
    
    fn commit_changes(&mut self) {
        if let Some(img) = &self.current_image {
            self.history_manager.push_state(img.clone());
        }
    }
    
    fn handle_zoom_input(&mut self, ctx: &egui::Context) {
        let scroll = ctx.input(|i| i.scroll_delta);
        let mods = ctx.input(|i| i.modifiers);
        if mods.command && scroll.y != 0.0 {
            let factor = 1.0 + scroll.y * 0.01;
            self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
        }
    }
    
    fn apply_theme(&self, ctx: &egui::Context) {
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
    }
    
    fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    let supported_extensions = self.raw_loader.get_supported_extensions();
                    if let Some(path) = FileDialog::new()
                        .add_filter("Images & RAW", &supported_extensions)
                        .pick_file()
                    {
                        self.load_image(path);
                    }
                }
                
                ui.separator();
                
                let can_undo = self.history_manager.can_undo();
                let can_redo = self.history_manager.can_redo();
                
                if ui.add_enabled(can_undo, egui::Button::new("Undo")).clicked() {
                    self.handle_undo();
                }
                if ui.add_enabled(can_redo, egui::Button::new("Redo")).clicked() {
                    self.handle_redo();
                }
                if ui.add_enabled(self.current_image.is_some(), egui::Button::new("Reset")).clicked() {
                    self.handle_reset();
                }
                
                ui.separator();
                
                ComboBox::from_label("Theme")
                    .selected_text(THEME_NAMES[self.theme])
                    .show_ui(ui, |ui| {
                        for (i, &name) in THEME_NAMES.iter().enumerate() {
                            ui.selectable_value(&mut self.theme, i, name);
                        }
                    });
                
                ui.separator();
                
                // Show image info if available
                if let Some(_img) = &self.current_image {
                    ui.label(format!("Zoom: {:.1}%", self.zoom * 100.0));
                }
            });
        });
    }
    
    fn render_adjustment_panel(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;
        
        egui::SidePanel::right("adjustment_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Adjustments");
                
                ui.separator();
                
                // Basic adjustments
                ui.label("Basic");
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.exposure, -5.0..=5.0)
                    .text("Exposure")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.contrast, -100.0..=100.0)
                    .text("Contrast")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.highlights, -100.0..=100.0)
                    .text("Highlights")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.shadows, -100.0..=100.0)
                    .text("Shadows")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.whites, -100.0..=100.0)
                    .text("Whites")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.blacks, -100.0..=100.0)
                    .text("Blacks")).changed();
                
                ui.separator();
                
                // Color adjustments
                ui.label("Color");
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.saturation, -100.0..=100.0)
                    .text("Saturation")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.vibrance, -100.0..=100.0)
                    .text("Vibrance")).changed();
                
                ui.separator();
                
                // White balance
                ui.label("White Balance");
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.temperature, -100.0..=100.0)
                    .text("Temperature")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.tint, -100.0..=100.0)
                    .text("Tint")).changed();
                
                ui.separator();
                
                // Advanced adjustments
                ui.label("Advanced");
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.clarity, -100.0..=100.0)
                    .text("Clarity")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.adjustment_state.dehaze, -100.0..=100.0)
                    .text("Dehaze")).changed();
                
                ui.separator();
                
                // Reset button
                if ui.button("Reset All").clicked() {
                    self.adjustment_state.reset();
                    changed = true;
                }
                
                // Commit changes button
                if ui.button("Apply Changes").clicked() {
                    self.commit_changes();
                }
            });
        
        changed
    }
    
    fn render_main_panel(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let size = tex.size_vec2() * self.zoom;
                        let response = ui.image((tex.id(), size));
                        
                        // Show image coordinates on hover
                        if response.hovered() {
                            if let Some(pos) = response.hover_pos() {
                                let image_pos = (pos - response.rect.min) / self.zoom;
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                                egui::show_tooltip_at_pointer(ui.ctx(), egui::Id::new("image_coords"), |ui| {
                                    ui.label(format!("X: {:.0}, Y: {:.0}", image_pos.x, image_pos.y));
                                });
                            }
                        }
                    });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Obsidian RAW Editor");
                        ui.add_space(20.0);
                        ui.label("Open an image or RAW file to get started");
                        ui.add_space(10.0);
                        ui.label("Supported formats:");
                        ui.label("• RAW: CR2, NEF, ARW, DNG, RAF, ORF, RW2, and more");
                        ui.label("• Standard: JPEG, PNG, TIFF, BMP, WebP");
                    });
                });
            }
        });
    }
}

impl App for ObsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Handle zoom input
        self.handle_zoom_input(ctx);
        
        // Receive processing results
        if let Ok(result) = self.result_receiver.try_recv() {
            match result {
                ProcessingResult::Success(color_image) => {
                    let tex = ctx.load_texture("main_image", color_image, TextureOptions::default());
                    self.texture = Some(tex);
                }
                ProcessingResult::Error(e) => {
                    eprintln!("Processing error: {}", e);
                }
            }
        }
        
        // Apply theme
        self.apply_theme(ctx);
        
        // Render UI panels
        self.render_top_panel(ctx);
        let adjustments_changed = self.render_adjustment_panel(ctx);
        self.render_main_panel(ctx);
        
        // Queue processing job if adjustments changed
        if adjustments_changed {
            self.queue_processing_job();
        }
    }
}

fn main() {
    let native_options = NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1200.0, 800.0)),
        ..Default::default()
    };
    
    run_native(
        "Obsidian RAW Editor",
        native_options,
        Box::new(|_cc| Box::new(ObsApp::default())),
    ).unwrap();
}