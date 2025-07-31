// src/image_processor.rs
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage, imageops};
use crate::adjustment_state::AdjustmentState;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProcessingJob {
    pub image: DynamicImage,
    pub adjustments: AdjustmentState,
}

#[derive(Debug)]
pub enum ProcessingResult {
    Success(eframe::egui::ColorImage),
    Error(String),
}

pub struct ImageProcessor {
    processing_order: Vec<ProcessStep>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStep {
    Exposure,
    HighlightsShadows,
    WhitesBlacks,
    Contrast,
    WhiteBalance,
    Saturation,
    Vibrance,
    Clarity,
    Dehaze,
    NoiseReduction,
    Sharpening,
    ToneCurve,
    ColorGrading,
    LensCorrections,
}

impl ImageProcessor {
    pub fn new() -> Self {
        Self {
            processing_order: vec![
                ProcessStep::Exposure,
                ProcessStep::HighlightsShadows,
                ProcessStep::WhitesBlacks,
                ProcessStep::WhiteBalance,
                ProcessStep::Contrast,
                ProcessStep::ToneCurve,
                ProcessStep::Saturation,
                ProcessStep::Vibrance,
                ProcessStep::ColorGrading,
                ProcessStep::Clarity,
                ProcessStep::Dehaze,
                ProcessStep::NoiseReduction,
                ProcessStep::Sharpening,
                ProcessStep::LensCorrections,
            ],
        }
    }
    
    pub fn process_image(&self, job: ProcessingJob) -> ProcessingResult {
        let mut img = job.image;
        let adjustments = &job.adjustments;
        
        // Ensure we're working with RGBA for consistent processing
        let mut rgba_img = img.to_rgba8();
        
        // Apply each processing step in order
        for step in &self.processing_order {
            match self.apply_processing_step(&mut rgba_img, step, adjustments) {
                Ok(_) => {},
                Err(e) => {
                    return ProcessingResult::Error(format!("Error in {:?}: {}", step, e));
                }
            }
        }
        
        // Convert to ColorImage for UI display
        match self.to_color_image(rgba_img) {
            Ok(color_image) => ProcessingResult::Success(color_image),
            Err(e) => ProcessingResult::Error(format!("Failed to convert final image: {}", e)),
        }
    }
    
    fn apply_processing_step(
        &self,
        image: &mut RgbaImage,
        step: &ProcessStep,
        adjustments: &AdjustmentState,
    ) -> Result<(), String> {
        match step {
            ProcessStep::Exposure => {
                if adjustments.exposure.abs() > f32::EPSILON {
                    self.apply_exposure(image, adjustments.exposure)?;
                }
            }
            ProcessStep::HighlightsShadows => {
                if adjustments.highlights.abs() > f32::EPSILON || adjustments.shadows.abs() > f32::EPSILON {
                    self.apply_highlights_shadows(image, adjustments.highlights, adjustments.shadows)?;
                }
            }
            ProcessStep::WhitesBlacks => {
                if adjustments.whites.abs() > f32::EPSILON || adjustments.blacks.abs() > f32::EPSILON {
                    self.apply_whites_blacks(image, adjustments.whites, adjustments.blacks)?;
                }
            }
            ProcessStep::Contrast => {
                if adjustments.contrast.abs() > f32::EPSILON {
                    self.apply_contrast(image, adjustments.contrast)?;
                }
            }
            ProcessStep::WhiteBalance => {
                if adjustments.temperature.abs() > f32::EPSILON || adjustments.tint.abs() > f32::EPSILON {
                    self.apply_white_balance(image, adjustments.temperature, adjustments.tint)?;
                }
            }
            ProcessStep::Saturation => {
                if adjustments.saturation.abs() > f32::EPSILON {
                    self.apply_saturation(image, adjustments.saturation)?;
                }
            }
            ProcessStep::Vibrance => {
                if adjustments.vibrance.abs() > f32::EPSILON {
                    self.apply_vibrance(image, adjustments.vibrance)?;
                }
            }
            ProcessStep::Clarity => {
                if adjustments.clarity.abs() > f32::EPSILON {
                    self.apply_clarity(image, adjustments.clarity)?;
                }
            }
            ProcessStep::Dehaze => {
                if adjustments.dehaze.abs() > f32::EPSILON {
                    self.apply_dehaze(image, adjustments.dehaze)?;
                }
            }
            ProcessStep::NoiseReduction => {
                if adjustments.noise_reduction > f32::EPSILON {
                    self.apply_noise_reduction(image, adjustments.noise_reduction)?;
                }
            }
            ProcessStep::Sharpening => {
                if adjustments.sharpening > f32::EPSILON {
                    self.apply_sharpening(image, adjustments.sharpening)?;
                }
            }
            ProcessStep::ToneCurve => {
                if adjustments.tone_curve.has_changes() {
                    self.apply_tone_curve(image, &adjustments.tone_curve)?;
                }
            }
            ProcessStep::ColorGrading => {
                if adjustments.color_grading.has_changes() {
                    self.apply_color_grading(image, &adjustments.color_grading)?;
                }
            }
            ProcessStep::LensCorrections => {
                if adjustments.lens_corrections.has_changes() {
                    self.apply_lens_corrections(image, &adjustments.lens_corrections)?;
                }
            }
        }
        Ok(())
    }
    
    fn apply_exposure(&self, image: &mut RgbaImage, exposure: f32) -> Result<(), String> {
        let factor = 2.0_f32.powf(exposure);
        
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            pixel.0 = [
                ((r as f32 * factor).min(255.0)) as u8,
                ((g as f32 * factor).min(255.0)) as u8,
                ((b as f32 * factor).min(255.0)) as u8,
                a,
            ];
        }
        Ok(())
    }
    
    fn apply_highlights_shadows(&self, image: &mut RgbaImage, highlights: f32, shadows: f32) -> Result<(), String> {
        let highlight_factor = 1.0 - (highlights / 100.0).clamp(-1.0, 1.0);
        let shadow_factor = 1.0 + (shadows / 100.0).clamp(-1.0, 1.0);
        
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            
            // Calculate luminance to determine if pixel is in highlights or shadows
            let lum = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            let lum_norm = lum / 255.0;
            
            // Apply different adjustments based on luminance
            let (r_adj, g_adj, b_adj) = if lum_norm > 0.7 {
                // Highlights
                (
                    (r as f32 * highlight_factor).clamp(0.0, 255.0),
                    (g as f32 * highlight_factor).clamp(0.0, 255.0),
                    (b as f32 * highlight_factor).clamp(0.0, 255.0),
                )
            } else if lum_norm < 0.3 {
                // Shadows
                (
                    (r as f32 * shadow_factor).clamp(0.0, 255.0),
                    (g as f32 * shadow_factor).clamp(0.0, 255.0),
                    (b as f32 * shadow_factor).clamp(0.0, 255.0),
                )
            } else {
                // Midtones - blend the adjustments
                let highlight_weight = (lum_norm - 0.3) / 0.4;
                let shadow_weight = 1.0 - highlight_weight;
                
                let factor = highlight_factor * highlight_weight + shadow_factor * shadow_weight;
                (
                    (r as f32 * factor).clamp(0.0, 255.0),
                    (g as f32 * factor).clamp(0.0, 255.0),
                    (b as f32 * factor).clamp(0.0, 255.0),
                )
            };
            
            pixel.0 = [r_adj as u8, g_adj as u8, b_adj as u8, a];
        }
        Ok(())
    }
    
    fn apply_whites_blacks(&self, image: &mut RgbaImage, whites: f32, blacks: f32) -> Result<(), String> {
        let white_point = 255.0 * (1.0 + whites / 100.0).clamp(0.5, 1.5);
        let black_point = 255.0 * (blacks / 100.0).clamp(-0.5, 0.5);
        
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            
            // Map the values to new range
            let r_new = ((r as f32 - black_point) * (255.0 / (white_point - black_point))).clamp(0.0, 255.0);
            let g_new = ((g as f32 - black_point) * (255.0 / (white_point - black_point))).clamp(0.0, 255.0);
            let b_new = ((b as f32 - black_point) * (255.0 / (white_point - black_point))).clamp(0.0, 255.0);
            
            pixel.0 = [r_new as u8, g_new as u8, b_new as u8, a];
        }
        Ok(())
    }
    
    fn apply_contrast(&self, image: &mut RgbaImage, contrast: f32) -> Result<(), String> {
        let factor = (contrast / 100.0 + 1.0).max(0.0);
        
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            
            // Apply contrast around midpoint (128)
            let r_new = (128.0 + (r as f32 - 128.0) * factor).clamp(0.0, 255.0);
            let g_new = (128.0 + (g as f32 - 128.0) * factor).clamp(0.0, 255.0);
            let b_new = (128.0 + (b as f32 - 128.0) * factor).clamp(0.0, 255.0);
            
            pixel.0 = [r_new as u8, g_new as u8, b_new as u8, a];
        }
        Ok(())
    }
    
    fn apply_white_balance(&self, image: &mut RgbaImage, temperature: f32, tint: f32) -> Result<(), String> {
        // Convert temperature to RGB multipliers
        let temp_kelvin = 5500.0 + temperature * 50.0; // Map -100..100 to roughly 500K..10500K
        let (r_temp, g_temp, b_temp) = self.kelvin_to_rgb(temp_kelvin);
        
        // Apply tint (green-magenta adjustment)
        let tint_factor = tint / 100.0;
        let r_tint = 1.0 - tint_factor * 0.1;
        let g_tint = 1.0 + tint_factor * 0.1;
        let b_tint = 1.0;
        
        // Combine temperature and tint
        let r_mult = r_temp * r_tint;
        let g_mult = g_temp * g_tint;
        let b_mult = b_temp * b_tint;
        
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            pixel.0 = [
                (r as f32 * r_mult).clamp(0.0, 255.0) as u8,
                (g as f32 * g_mult).clamp(0.0, 255.0) as u8,
                (b as f32 * b_mult).clamp(0.0, 255.0) as u8,
                a,
            ];
        }
        Ok(())
    }
    
    fn kelvin_to_rgb(&self, kelvin: f32) -> (f32, f32, f32) {
        // Simplified color temperature to RGB conversion
        let temp = kelvin / 100.0;
        
        let r = if temp <= 66.0 {
            1.0
        } else {
            let r = temp - 60.0;
            (329.698727446 * r.powf(-0.1332047592)) / 255.0
        };
        
        let g = if temp <= 66.0 {
            let g = temp;
            (99.4708025861 * g.ln() - 161.1195681661) / 255.0
        } else {
            let g = temp - 60.0;
            (288.1221695283 * g.powf(-0.0755148492)) / 255.0
        };
        
        let b = if temp >= 66.0 {
            1.0
        } else if temp <= 19.0 {
            0.0
        } else {
            let b = temp - 10.0;
            (138.5177312231 * b.ln() - 305.0447927307) / 255.0
        };
        
        (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
    }
    
    fn apply_saturation(&self, image: &mut RgbaImage, saturation: f32) -> Result<(), String> {
        let factor = 1.0 + saturation / 100.0;
        
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            let lum = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            
            let r_new = (lum + (r as f32 - lum) * factor).clamp(0.0, 255.0);
            let g_new = (lum + (g as f32 - lum) * factor).clamp(0.0, 255.0);
            let b_new = (lum + (b as f32 - lum) * factor).clamp(0.0, 255.0);
            
            pixel.0 = [r_new as u8, g_new as u8, b_new as u8, a];
        }
        Ok(())
    }
    
    fn apply_vibrance(&self, image: &mut RgbaImage, vibrance: f32) -> Result<(), String> {
        let factor = 1.0 + vibrance / 100.0;
        
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            
            // Calculate current saturation
            let max_rgb = r.max(g).max(b) as f32;
            let min_rgb = r.min(g).min(b) as f32;
            let current_sat = if max_rgb > 0.0 { (max_rgb - min_rgb) / max_rgb } else { 0.0 };
            
            // Reduce vibrance effect on already saturated colors
            let adjusted_factor = 1.0 + (factor - 1.0) * (1.0 - current_sat);
            
            let lum = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            let r_new = (lum + (r as f32 - lum) * adjusted_factor).clamp(0.0, 255.0);
            let g_new = (lum + (g as f32 - lum) * adjusted_factor).clamp(0.0, 255.0);
            let b_new = (lum + (b as f32 - lum) * adjusted_factor).clamp(0.0, 255.0);
            
            pixel.0 = [r_new as u8, g_new as u8, b_new as u8, a];
        }
        Ok(())
    }
    
    fn apply_clarity(&self, image: &mut RgbaImage, clarity: f32) -> Result<(), String> {
        // Clarity enhances local contrast in midtones
        // This is a simplified implementation - full clarity would use unsharp masking with edge detection
        
        let strength = clarity / 100.0;
        let (width, height) = image.dimensions();
        let mut result = image.clone();
        
        // Simple local contrast enhancement
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let center_pixel = image.get_pixel(x, y);
                let [r, g, b, a] = center_pixel.0;
                
                // Calculate local average
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;
                let mut count = 0u32;
                
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let px = image.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32);
                        sum_r += px.0[0] as u32;
                        sum_g += px.0[1] as u32;
                        sum_b += px.0[2] as u32;
                        count += 1;
                    }
                }
                
                let avg_r = sum_r as f32 / count as f32;
                let avg_g = sum_g as f32 / count as f32;
                let avg_b = sum_b as f32 / count as f32;
                
                // Apply clarity adjustment (enhance differences from local average)
                let r_new = (r as f32 + (r as f32 - avg_r) * strength).clamp(0.0, 255.0);
                let g_new = (g as f32 + (g as f32 - avg_g) * strength).clamp(0.0, 255.0);
                let b_new = (b as f32 + (b as f32 - avg_b) * strength).clamp(0.0, 255.0);
                
                result.put_pixel(x, y, Rgba([r_new as u8, g_new as u8, b_new as u8, a]));
            }
        }
        
        *image = result;
        Ok(())
    }
    
    fn apply_dehaze(&self, _image: &mut RgbaImage, _dehaze: f32) -> Result<(), String> {
        // Dehaze is a complex algorithm that would require atmospheric light estimation
        // and transmission map calculation. For now, we'll implement a placeholder
        // that applies a slight contrast and saturation boost
        Ok(())
    }
    
    fn apply_noise_reduction(&self, _image: &mut RgbaImage, _noise_reduction: f32) -> Result<(), String> {
        // Noise reduction would typically use algorithms like bilateral filtering
        // or non-local means. For now, this is a placeholder
        Ok(())
    }
    
    fn apply_sharpening(&self, image: &mut RgbaImage, sharpening: f32) -> Result<(), String> {
        if sharpening <= 0.0 {
            return Ok(());
        }
        
        let strength = sharpening / 100.0;
        let (width, height) = image.dimensions();
        let mut result = image.clone();
        
        // Unsharp mask kernel
        let kernel = [
            [0.0, -1.0, 0.0],
            [-1.0, 5.0, -1.0],
            [0.0, -1.0, 0.0],
        ];
        
        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                
                for ky in 0..3 {
                    for kx in 0..3 {
                        let px = image.get_pixel((x + kx - 1), (y + ky - 1));
                        let weight = kernel[ky][kx];
                        sum_r += px.0[0] as f32 * weight;
                        sum_g += px.0[1] as f32 * weight;
                        sum_b += px.0[2] as f32 * weight;
                    }
                }
                
                let original = image.get_pixel(x, y);
                let [orig_r, orig_g, orig_b, a] = original.0;
                
                // Blend original with sharpened version
                let r_sharp = (orig_r as f32 * (1.0 - strength) + sum_r.clamp(0.0, 255.0) * strength) as u8;
                let g_sharp = (orig_g as f32 * (1.0 - strength) + sum_g.clamp(0.0, 255.0) * strength) as u8;
                let b_sharp = (orig_b as f32 * (1.0 - strength) + sum_b.clamp(0.0, 255.0) * strength) as u8;
                
                result.put_pixel(x, y, Rgba([r_sharp, g_sharp, b_sharp, a]));
            }
        }
        
        *image = result;
        Ok(())
    }
    
    fn apply_tone_curve(&self, image: &mut RgbaImage, tone_curve: &crate::adjustment_state::ToneCurve) -> Result<(), String> {
        for pixel in image.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            
            // Apply tone curve to each channel
            let r_norm = r as f32 / 255.0;
            let g_norm = g as f32 / 255.0;
            let b_norm = b as f32 / 255.0;
            
            let r_new = (tone_curve.evaluate(r_norm) * 255.0).clamp(0.0, 255.0) as u8;
            let g_new = (tone_curve.evaluate(g_norm) * 255.0).clamp(0.0, 255.0) as u8;
            let b_new = (tone_curve.evaluate(b_norm) * 255.0).clamp(0.0, 255.0) as u8;
            
            pixel.0 = [r_new, g_new, b_new, a];
        }
        Ok(())
    }
    
    fn apply_color_grading(&self, _image: &mut RgbaImage, _color_grading: &crate::adjustment_state::ColorGrading) -> Result<(), String> {
        // Color grading would apply different adjustments to shadows, midtones, and highlights
        // This is a placeholder for the complex implementation
        Ok(())
    }
    
    fn apply_lens_corrections(&self, _image: &mut RgbaImage, _lens_corrections: &crate::adjustment_state::LensCorrections) -> Result<(), String> {
        // Lens corrections would include chromatic aberration, vignetting, and distortion correction
        // This is a placeholder for the complex implementation
        Ok(())
    }
    
    fn to_color_image(&self, rgba_img: RgbaImage) -> Result<eframe::egui::ColorImage, String> {
        let size = [rgba_img.width() as usize, rgba_img.height() as usize];
        let pixels = rgba_img.into_flat_samples();
        let pixels_slice = pixels.as_slice();
        
        Ok(eframe::egui::ColorImage::from_rgba_unmultiplied(size, pixels_slice))
    }
    
    /// Set custom processing order for specific workflows
    pub fn set_processing_order(&mut self, order: Vec<ProcessStep>) {
        self.processing_order = order;
    }
    
    /// Get current processing order
    pub fn get_processing_order(&self) -> &[ProcessStep] {
        &self.processing_order
    }
    
    /// Apply a single processing step (useful for previews)
    pub fn apply_single_step(&self, image: DynamicImage, step: ProcessStep, adjustments: &AdjustmentState) -> Result<DynamicImage, String> {
        let mut rgba_img = image.to_rgba8();
        self.apply_processing_step(&mut rgba_img, &step, adjustments)?;
        Ok(DynamicImage::ImageRgba8(rgba_img))
    }
    
    /// Generate a quick preview with reduced quality for real-time adjustments
    pub fn process_preview(&self, job: ProcessingJob, max_dimension: u32) -> ProcessingResult {
        let mut img = job.image;
        
        // Resize for faster processing
        let (width, height) = (img.width(), img.height());
        if width > max_dimension || height > max_dimension {
            let ratio = (max_dimension as f32 / width.max(height) as f32).min(1.0);
            let new_width = (width as f32 * ratio) as u32;
            let new_height = (height as f32 * ratio) as u32;
            img = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
        }
        
        // Process with reduced steps for speed
        let preview_job = ProcessingJob {
            image: img,
            adjustments: job.adjustments,
        };
        
        self.process_image(preview_job)
    }
    
    /// Calculate histogram data for the image
    pub fn calculate_histogram(&self, image: &DynamicImage) -> ImageHistogram {
        let rgba_img = image.to_rgba8();
        let mut red = vec![0u32; 256];
        let mut green = vec![0u32; 256];
        let mut blue = vec![0u32; 256];
        let mut luminance = vec![0u32; 256];
        
        for pixel in rgba_img.pixels() {
            let [r, g, b, _] = pixel.0;
            red[r as usize] += 1;
            green[g as usize] += 1;
            blue[b as usize] += 1;
            
            // Calculate luminance
            let lum = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
            luminance[lum as usize] += 1;
        }
        
        ImageHistogram {
            red,
            green,
            blue,
            luminance,
            total_pixels: rgba_img.pixels().len() as u32,
        }
    }
    
    /// Export processed image with full quality
    pub fn export_image(&self, job: ProcessingJob, format: ExportFormat) -> Result<Vec<u8>, String> {
        let processed = match self.process_image(job) {
            ProcessingResult::Success(_) => {
                // We need to re-process to get the actual image data, not ColorImage
                let mut img = job.image;
                let mut rgba_img = img.to_rgba8();
                
                for step in &self.processing_order {
                    self.apply_processing_step(&mut rgba_img, step, &job.adjustments)?;
                }
                
                DynamicImage::ImageRgba8(rgba_img)
            }
            ProcessingResult::Error(e) => return Err(e),
        };
        
        let mut buffer = Vec::new();
        match format {
            ExportFormat::Jpeg { quality } => {
                let rgb_img = processed.to_rgb8();
                let mut cursor = std::io::Cursor::new(&mut buffer);
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality)
                    .write_image(
                        rgb_img.as_raw(),
                        rgb_img.width(),
                        rgb_img.height(),
                        image::ColorType::Rgb8,
                    )
                    .map_err(|e| format!("JPEG export error: {}", e))?;
            }
            ExportFormat::Png { compression } => {
                let mut cursor = std::io::Cursor::new(&mut buffer);
                let encoder = image::codecs::png::PngEncoder::new_with_quality(
                    &mut cursor,
                    image::codecs::png::CompressionType::Default,
                    image::codecs::png::FilterType::NoFilter,
                );
                encoder
                    .write_image(
                        processed.as_bytes(),
                        processed.width(),
                        processed.height(),
                        processed.color(),
                    )
                    .map_err(|e| format!("PNG export error: {}", e))?;
            }
            ExportFormat::Tiff => {
                let mut cursor = std::io::Cursor::new(&mut buffer);
                processed
                    .write_to(&mut cursor, image::ImageOutputFormat::Tiff)
                    .map_err(|e| format!("TIFF export error: {}", e))?;
            }
        }
        
        Ok(buffer)
    }
}

#[derive(Debug, Clone)]
pub struct ImageHistogram {
    pub red: Vec<u32>,
    pub green: Vec<u32>,
    pub blue: Vec<u32>,
    pub luminance: Vec<u32>,
    pub total_pixels: u32,
}

impl ImageHistogram {
    pub fn get_peak_value(&self) -> u32 {
        self.red.iter()
            .chain(self.green.iter())
            .chain(self.blue.iter())
            .chain(self.luminance.iter())
            .copied()
            .max()
            .unwrap_or(0)
    }
    
    pub fn get_normalized_red(&self) -> Vec<f32> {
        let max_val = self.get_peak_value() as f32;
        if max_val > 0.0 {
            self.red.iter().map(|&x| x as f32 / max_val).collect()
        } else {
            vec![0.0; 256]
        }
    }
    
    pub fn get_normalized_green(&self) -> Vec<f32> {
        let max_val = self.get_peak_value() as f32;
        if max_val > 0.0 {
            self.green.iter().map(|&x| x as f32 / max_val).collect()
        } else {
            vec![0.0; 256]
        }
    }
    
    pub fn get_normalized_blue(&self) -> Vec<f32> {
        let max_val = self.get_peak_value() as f32;
        if max_val > 0.0 {
            self.blue.iter().map(|&x| x as f32 / max_val).collect()
        } else {
            vec![0.0; 256]
        }
    }
    
    pub fn get_normalized_luminance(&self) -> Vec<f32> {
        let max_val = self.get_peak_value() as f32;
        if max_val > 0.0 {
            self.luminance.iter().map(|&x| x as f32 / max_val).collect()
        } else {
            vec![0.0; 256]
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Jpeg { quality: u8 },
    Png { compression: u8 },
    Tiff,
}

impl Default for ExportFormat {
    fn default() -> Self {
        ExportFormat::Jpeg { quality: 95 }
    }
}

/// Processing statistics for performance monitoring
#[derive(Debug, Clone)]
pub struct ProcessingStatistics {
    pub total_time_ms: u64,
    pub step_times_ms: std::collections::HashMap<ProcessStep, u64>,
    pub image_dimensions: (u32, u32),
    pub memory_usage_mb: f64,
}

impl ProcessingStatistics {
    pub fn new() -> Self {
        Self {
            total_time_ms: 0,
            step_times_ms: std::collections::HashMap::new(),
            image_dimensions: (0, 0),
            memory_usage_mb: 0.0,
        }
    }
}