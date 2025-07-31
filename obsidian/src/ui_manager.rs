// src/ui_manager.rs
use eframe::egui::{self, ColorImage, TextureOptions, Color32, ComboBox, CursorIcon};
use rfd::FileDialog;
use std::path::PathBuf;
use crate::adjustment_state::AdjustmentState;
use crate::raw_loader::RawLoader;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    ObsidianDark = 0,
    ObsidianLight = 1,
    PurpleDark = 2,
    SolarizedLight = 3,
}

impl Theme {
    pub const ALL: &'static [Theme] = &[
        Theme::ObsidianDark,
        Theme::ObsidianLight,
        Theme::PurpleDark,
        Theme::SolarizedLight,
    ];
    
    pub fn name(&self) -> &'static str {
        match self {
            Theme::ObsidianDark => "Obsidian Dark",
            Theme::ObsidianLight => "Obsidian Light",
            Theme::PurpleDark => "Purple Dark",
            Theme::SolarizedLight => "Solarized Light",
        }
    }
    
    pub fn from_index(index: usize) -> Theme {
        match index {
            0 => Theme::ObsidianDark,
            1 => Theme::ObsidianLight,
            2 => Theme::PurpleDark,
            3 => Theme::SolarizedLight,
            _ => Theme::ObsidianDark,
        }
    }
    
    pub fn to_index(&self) -> usize {
        *self as usize
    }
}

pub enum TopPanelAction {
    OpenFile(PathBuf),
    Undo,
    Redo,
    Reset,
    ThemeChanged(Theme),
    Export,
}

pub enum MainPanelAction {
    ImageClicked { x: f32, y: f32 },
    ZoomChanged(f32),
}

pub struct UIState {
    pub zoom: f32,
    pub theme: Theme,
    pub show_histogram: bool,
    pub show_info_panel: bool,
    pub adjustment_panel_width: f32,
    pub current_tool: Tool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tool {
    None,
    CropTool,
    SpotRemoval,
    LocalAdjustment,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            theme: Theme::ObsidianDark,
            show_histogram: false,
            show_info_panel: false,
            adjustment_panel_width: 280.0,
            current_tool: Tool::None,
        }
    }
}

pub struct UIManager {
    state: UIState,
    raw_loader: RawLoader,
}

impl UIManager {
    pub fn new() -> Self {
        Self {
            state: UIState::default(),
            raw_loader: RawLoader::new(),
        }
    }
    
    pub fn get_zoom(&self) -> f32 {
        self.state.zoom
    }
    
    pub fn set_zoom(&mut self, zoom: f32) {
        self.state.zoom = zoom.clamp(0.1, 10.0);
    }
    
    pub fn get_theme(&self) -> Theme {
        self.state.theme
    }
    
    pub fn handle_zoom_input(&mut self, ctx: &egui::Context) -> Option<f32> {
        let scroll = ctx.input(|i| i.scroll_delta);
        let mods = ctx.input(|i| i.modifiers);
        
        if mods.command && scroll.y != 0.0 {
            let factor = 1.0 + scroll.y * 0.01;
            let new_zoom = (self.state.zoom * factor).clamp(0.1, 10.0);
            if (new_zoom - self.state.zoom).abs() > f32::EPSILON {
                self.state.zoom = new_zoom;
                return Some(new_zoom);
            }
        }
        None
    }
    
    pub fn apply_theme(&self, ctx: &egui::Context) {
        match self.state.theme {
            Theme::ObsidianDark => {
                let mut visuals = egui::Visuals::dark();
                visuals.panel_fill = Color32::from_rgb(25, 25, 25);
                visuals.window_fill = Color32::from_rgb(30, 30, 30);
                visuals.faint_bg_color = Color32::from_rgb(35, 35, 35);
                ctx.set_visuals(visuals);
            }
            Theme::ObsidianLight => {
                let mut visuals = egui::Visuals::light();
                visuals.panel_fill = Color32::from_rgb(248, 248, 248);
                visuals.window_fill = Color32::from_rgb(255, 255, 255);
                ctx.set_visuals(visuals);
            }
            Theme::PurpleDark => {
                let mut visuals = egui::Visuals::dark();
                visuals.panel_fill = Color32::from_rgb(40, 30, 80);
                visuals.window_fill = Color32::from_rgb(45, 35, 85);
                visuals.faint_bg_color = Color32::from_rgb(50, 40, 90);
                visuals.selection.bg_fill = Color32::from_rgb(120, 80, 160);
                ctx.set_visuals(visuals);
            }
            Theme::SolarizedLight => {
                let mut visuals = egui::Visuals::light();
                visuals.panel_fill = Color32::from_rgb(253, 246, 227);
                visuals.window_fill = Color32::from_rgb(238, 232, 213);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(250, 240, 210);
                visuals.selection.bg_fill = Color32::from_rgb(181, 137, 0);
                ctx.set_visuals(visuals);
            }
        }
    }
    
    pub fn render_top_panel<F>(&mut self, ctx: &egui::Context, mut on_action: F)
    where
        F: FnMut(TopPanelAction),
    {
        egui::TopBottomPanel::top("top_panel")
            .exact_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    
                    // File operations
                    if ui.button("📁 Open").clicked() {
                        let supported_extensions = self.raw_loader.get_supported_extensions();
                        if let Some(path) = FileDialog::new()
                            .add_filter("Images & RAW", &supported_extensions)
                            .pick_file()
                        {
                            on_action(TopPanelAction::OpenFile(path));
                        }
                    }
                    
                    if ui.button("💾 Export").clicked() {
                        on_action(TopPanelAction::Export);
                    }
                    
                    ui.separator();
                    
                    // Edit operations
                    if ui.button("↶ Undo").clicked() {
                        on_action(TopPanelAction::Undo);
                    }
                    if ui.button("↷ Redo").clicked() {
                        on_action(TopPanelAction::Redo);
                    }
                    if ui.button("🔄 Reset").clicked() {
                        on_action(TopPanelAction::Reset);
                    }
                    
                    ui.separator();
                    
                    // Tools
                    ui.label("Tools:");
                    ui.selectable_value(&mut self.state.current_tool, Tool::None, "Select");
                    ui.selectable_value(&mut self.state.current_tool, Tool::CropTool, "Crop");
                    ui.selectable_value(&mut self.state.current_tool, Tool::SpotRemoval, "Spot");
                    ui.selectable_value(&mut self.state.current_tool, Tool::LocalAdjustment, "Local");
                    
                    ui.separator();
                    
                    // View options
                    ui.checkbox(&mut self.state.show_histogram, "📊 Histogram");
                    ui.checkbox(&mut self.state.show_info_panel, "ℹ Info");
                    
                    ui.separator();
                    
                    // Theme selector
                    let current_theme_name = self.state.theme.name();
                    ComboBox::from_label("🎨")
                        .selected_text(current_theme_name)
                        .show_ui(ui, |ui| {
                            for &theme in Theme::ALL {
                                if ui.selectable_value(&mut self.state.theme, theme, theme.name()).changed() {
                                    on_action(TopPanelAction::ThemeChanged(theme));
                                }
                            }
                        });
                    
                    // Zoom indicator (right-aligned)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("🔍 {:.0}%", self.state.zoom * 100.0));
                    });
                });
            });
    }
    
    pub fn render_adjustment_panel(&mut self, ctx: &egui::Context, adjustments: &mut AdjustmentState) -> bool {
        let mut changed = false;
        
        egui::SidePanel::right("adjustment_panel")
            .resizable(true)
            .default_width(self.state.adjustment_panel_width)
            .width_range(200.0..=400.0)
            .show(ctx, |ui| {
                // Store the actual width
                self.state.adjustment_panel_width = ui.available_width();
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("🎛️ Adjustments");
                    ui.separator();
                    
                    // Basic panel
                    ui.collapsing("📷 Basic", |ui| {
                        ui.spacing_mut().slider_width = ui.available_width() - 80.0;
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.exposure, -5.0..=5.0)
                                .text("Exposure")
                                .suffix(" EV")
                        ).changed();
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.contrast, -100.0..=100.0)
                                .text("Contrast")
                        ).changed();
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.highlights, -100.0..=100.0)
                                .text("Highlights")
                        ).changed();
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.shadows, -100.0..=100.0)
                                .text("Shadows")
                        ).changed();
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.whites, -100.0..=100.0)
                                .text("Whites")
                        ).changed();
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.blacks, -100.0..=100.0)
                                .text("Blacks")
                        ).changed();
                    });
                    
                    ui.separator();
                    
                    // Color panel
                    ui.collapsing("🌈 Color", |ui| {
                        ui.spacing_mut().slider_width = ui.available_width() - 80.0;
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.saturation, -100.0..=100.0)
                                .text("Saturation")
                        ).changed();
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.vibrance, -100.0..=100.0)
                                .text("Vibrance")
                        ).changed();
                    });
                    
                    ui.separator();
                    
                    // White Balance panel
                    ui.collapsing("🔆 White Balance", |ui| {
                        ui.spacing_mut().slider_width = ui.available_width() - 80.0;
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.temperature, -100.0..=100.0)
                                .text("Temperature")
                                .suffix("K")
                        ).changed();
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.tint, -100.0..=100.0)
                                .text("Tint")
                        ).changed();
                    });
                    
                    ui.separator();
                    
                    // Advanced panel
                    ui.collapsing("⚡ Advanced", |ui| {
                        ui.spacing_mut().slider_width = ui.available_width() - 80.0;
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.clarity, -100.0..=100.0)
                                .text("Clarity")
                        ).changed();
                        
                        changed |= ui.add(
                            egui::Slider::new(&mut adjustments.dehaze, -100.0..=100.0)
                                .text("Dehaze")
                        ).changed();
                    });
                    
                    ui.separator();
                    
                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("🔄 Reset All").clicked() {
                            adjustments.reset();
                            changed = true;
                        }
                        
                        if ui.button("✅ Apply").clicked() {
                            // This would trigger a commit in the main app
                        }
                    });
                    
                    ui.separator();
                    
                    // Adjustment info
                    if adjustments.has_changes() {
                        ui.label("⚠️ Unsaved changes");
                    } else {
                        ui.label("✅ No changes");
                    }
                });
            });
        
        changed
    }
    
    pub fn render_main_panel<F>(&mut self, ctx: &egui::Context, texture: &Option<egui::TextureHandle>, mut on_action: F)
    where
        F: FnMut(MainPanelAction),
    {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = texture {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let size = tex.size_vec2() * self.state.zoom;
                        let response = ui.image((tex.id(), size));
                        
                        // Handle mouse interactions
                        if response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let image_pos = (pos - response.rect.min) / self.state.zoom;
                                on_action(MainPanelAction::ImageClicked {
                                    x: image_pos.x,
                                    y: image_pos.y,
                                });
                            }
                        }
                        
                        // Show cursor information on hover
                        if response.hovered() {
                            ctx.set_cursor_icon(match self.state.current_tool {
                                Tool::None => CursorIcon::Default,
                                Tool::CropTool => CursorIcon::Crosshair,
                                Tool::SpotRemoval => CursorIcon::PointingHand,
                                Tool::LocalAdjustment => CursorIcon::Grab,
                            });
                            
                            if let Some(pos) = response.hover_pos() {
                                let image_pos = (pos - response.rect.min) / self.state.zoom;
                                egui::show_tooltip_at_pointer(ctx, egui::Id::new("image_coords"), |ui| {
                                    ui.label(format!("X: {:.0}, Y: {:.0}", image_pos.x, image_pos.y));
                                    ui.label(format!("Tool: {:?}", self.state.current_tool));
                                });
                            }
                        }
                    });
            } else {
                self.render_welcome_screen(ui);
            }
        });
    }
    
    pub fn render_histogram_panel(&self, ctx: &egui::Context, texture: &Option<egui::TextureHandle>) {
        if !self.state.show_histogram {
            return;
        }
        
        egui::Window::new("📊 Histogram")
            .default_width(300.0)
            .default_height(200.0)
            .show(ctx, |ui| {
                if texture.is_some() {
                    // Placeholder for histogram rendering
                    ui.label("Histogram would be displayed here");
                    ui.separator();
                    ui.label("📈 Red channel");
                    ui.label("📈 Green channel"); 
                    ui.label("📈 Blue channel");
                    ui.label("📈 Luminance");
                } else {
                    ui.label("No image loaded");
                }
            });
    }
    
    pub fn render_info_panel(&self, ctx: &egui::Context, texture: &Option<egui::TextureHandle>) {
        if !self.state.show_info_panel {
            return;
        }
        
        egui::Window::new("ℹ️ Image Info")
            .default_width(250.0)
            .show(ctx, |ui| {
                if let Some(tex) = texture {
                    let size = tex.size_vec2();
                    ui.label(format!("📐 Dimensions: {:.0} × {:.0}", size.x, size.y));
                    ui.label(format!("🔍 Zoom: {:.1}%", self.state.zoom * 100.0));
                    ui.separator();
                    ui.label("📷 EXIF Data");
                    ui.label("• ISO: N/A");
                    ui.label("• Aperture: N/A");
                    ui.label("• Shutter: N/A");
                    ui.label("• Focal Length: N/A");
                } else {
                    ui.label("No image loaded");
                }
            });
    }
    
    fn render_welcome_screen(&self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                
                // App title with emoji
                ui.heading("🌟 Obsidian RAW Editor");
                ui.add_space(30.0);
                
                // Welcome message
                ui.label("Open an image or RAW file to get started");
                ui.add_space(20.0);
                
                // Supported formats
                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("📁 Supported Formats");
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            ui.label("🎞️ RAW:");
                            ui.label("CR2, CR3, NEF, ARW, DNG, RAF, ORF, RW2");
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label("🖼️ Standard:");
                            ui.label("JPEG, PNG, TIFF, BMP, WebP");
                        });
                    });
                });
                
                ui.add_space(30.0);
                
                // Quick tips
                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("💡 Quick Tips");
                        ui.separator();
                        ui.label("• Cmd/Ctrl + Scroll to zoom");
                        ui.label("• Use adjustment panels for editing");
                        ui.label("• Try different themes in the top bar");
                    });
                });
            });
        });
    }
    
    pub fn should_show_histogram(&self) -> bool {
        self.state.show_histogram
    }
    
    pub fn should_show_info_panel(&self) -> bool {
        self.state.show_info_panel
    }
    
    pub fn get_current_tool(&self) -> Tool {
        self.state.current_tool
    }
}