// src/raw_loader.rs
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use rawloader::{decode_file, RawImageData};
use std::path::Path;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum LoadError {
    UnsupportedFormat(String),
    RawDecodeError(String),
    ImageOpenError(String),
    InvalidData(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::UnsupportedFormat(ext) => write!(f, "Unsupported format: {}", ext),
            LoadError::RawDecodeError(msg) => write!(f, "RAW decode error: {}", msg),
            LoadError::ImageOpenError(msg) => write!(f, "Image open error: {}", msg),
            LoadError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl Error for LoadError {}

pub struct RawLoader {
    supported_raw_formats: Vec<&'static str>,
    supported_standard_formats: Vec<&'static str>,
}

impl RawLoader {
    pub fn new() -> Self {
        Self {
            supported_raw_formats: vec![
                "cr2", "cr3",           // Canon
                "nef", "nrw",           // Nikon
                "arw", "srf", "sr2",    // Sony
                "dng",                  // Adobe/Generic
                "raf",                  // Fujifilm
                "orf",                  // Olympus
                "rw2",                  // Panasonic
                "pef", "ptx",           // Pentax
                "x3f",                  // Sigma
                "dcr", "kdc", "k25",    // Kodak
                "mrw",                  // Minolta
                "3fr",                  // Hasselblad
                "ari",                  // Arri
                "bay",                  // Casio
                "cap", "iiq", "eip",    // Phase One
                "dcs", "dcr",           // Kodak
                "fff",                  // Imacon
                "mef",                  // Mamiya
                "mos",                  // Leaf
                "nrw",                  // Nikon
                "raw", "rwl", "rw2",    // Various
            ],
            supported_standard_formats: vec![
                "jpg", "jpeg", "png", "tiff", "tif", "bmp", "gif", "webp"
            ],
        }
    }
    
    pub fn load_image<P: AsRef<Path>>(&self, path: P) -> Result<DynamicImage, LoadError> {
        let path = path.as_ref();
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| LoadError::UnsupportedFormat("No extension".to_string()))?;
        
        if self.is_raw_format(&extension) {
            self.load_raw_image(path)
        } else if self.is_standard_format(&extension) {
            self.load_standard_image(path)
        } else {
            Err(LoadError::UnsupportedFormat(extension))
        }
    }
    
    pub fn is_supported_format(&self, extension: &str) -> bool {
        let ext = extension.to_lowercase();
        self.is_raw_format(&ext) || self.is_standard_format(&ext)
    }
    
    pub fn get_supported_extensions(&self) -> Vec<String> {
        let mut all_formats = Vec::new();
        all_formats.extend(self.supported_raw_formats.iter().map(|&s| s.to_string()));
        all_formats.extend(self.supported_standard_formats.iter().map(|&s| s.to_string()));
        all_formats
    }
    
    fn is_raw_format(&self, extension: &str) -> bool {
        self.supported_raw_formats.contains(&extension.as_ref())
    }
    
    fn is_standard_format(&self, extension: &str) -> bool {
        self.supported_standard_formats.contains(&extension.as_ref())
    }
    
    fn load_raw_image<P: AsRef<Path>>(&self, path: P) -> Result<DynamicImage, LoadError> {
        let raw_image = decode_file(path.as_ref())
            .map_err(|e| LoadError::RawDecodeError(format!("Failed to decode RAW: {}", e)))?;
        
        // Convert RAW data to RGB
        let rgb_data = self.raw_to_rgb(&raw_image)?;
        
        // Apply basic demosaicing and color correction
        let processed_data = self.apply_basic_processing(&rgb_data, &raw_image)?;
        
        // Create RGBA image buffer
        let rgba_data = self.rgb_to_rgba(&processed_data, raw_image.width as u32, raw_image.height as u32)?;
        
        let image_buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
            raw_image.width as u32,
            raw_image.height as u32,
            rgba_data,
        ).ok_or_else(|| LoadError::InvalidData("Failed to create image buffer".to_string()))?;
        
        Ok(DynamicImage::ImageRgba8(image_buffer))
    }
    
    fn load_standard_image<P: AsRef<Path>>(&self, path: P) -> Result<DynamicImage, LoadError> {
        image::open(path.as_ref())
            .map_err(|e| LoadError::ImageOpenError(format!("Failed to open image: {}", e)))
    }
    
    fn raw_to_rgb(&self, raw_image: &rawloader::RawImage) -> Result<Vec<u16>, LoadError> {
        match &raw_image.data {
            RawImageData::RawU8(data) => {
                // Convert u8 to u16 for processing
                Ok(data.iter().map(|&x| (x as u16) << 8).collect())
            }
            RawImageData::RawU16(data) => {
                Ok(data.clone())
            }
        }
    }
    
    fn apply_basic_processing(&self, data: &[u16], raw_image: &rawloader::RawImage) -> Result<Vec<u16>, LoadError> {
        let mut processed = data.to_vec();
        
        // Apply white balance if available
        if let Some(wb) = &raw_image.wb_coeffs {
            if wb.len() >= 3 {
                self.apply_white_balance(&mut processed, wb, raw_image.width, raw_image.height)?;
            }
        }
        
        // Apply basic tone curve
        self.apply_basic_tone_curve(&mut processed);
        
        // Simple demosaicing for Bayer pattern (if needed)
        if raw_image.cfa.len() > 0 {
            processed = self.simple_demosaic(&processed, raw_image)?;
        }
        
        Ok(processed)
    }
    
    fn apply_white_balance(&self, data: &mut [u16], wb_coeffs: &[f32], width: usize, height: usize) -> Result<(), LoadError> {
        if wb_coeffs.len() < 3 {
            return Err(LoadError::InvalidData("Insufficient white balance coefficients".to_string()));
        }
        
        let pixels_per_row = width * 3; // Assuming RGB
        
        for y in 0..height {
            for x in 0..width {
                let base_idx = y * pixels_per_row + x * 3;
                if base_idx + 2 < data.len() {
                    // Apply white balance coefficients
                    data[base_idx] = ((data[base_idx] as f32 * wb_coeffs[0]).min(65535.0)) as u16;     // R
                    data[base_idx + 1] = ((data[base_idx + 1] as f32 * wb_coeffs[1]).min(65535.0)) as u16; // G
                    data[base_idx + 2] = ((data[base_idx + 2] as f32 * wb_coeffs[2]).min(65535.0)) as u16; // B
                }
            }
        }
        
        Ok(())
    }
    
    fn apply_basic_tone_curve(&self, data: &mut [u16]) {
        // Apply a basic gamma correction and tone curve
        for pixel in data.iter_mut() {
            let normalized = *pixel as f32 / 65535.0;
            // Apply gamma 2.2 correction
            let gamma_corrected = normalized.powf(1.0 / 2.2);
            // Simple S-curve for better contrast
            let s_curve = self.apply_s_curve(gamma_corrected);
            *pixel = (s_curve * 65535.0).min(65535.0) as u16;
        }
    }
    
    fn apply_s_curve(&self, x: f32) -> f32 {
        // Simple S-curve using cubic function
        if x < 0.5 {
            2.0 * x * x
        } else {
            1.0 - 2.0 * (1.0 - x) * (1.0 - x)
        }
    }
    
    fn simple_demosaic(&self, data: &[u16], raw_image: &rawloader::RawImage) -> Result<Vec<u16>, LoadError> {
        // This is a very basic demosaicing implementation
        // In a production system, you'd want more sophisticated algorithms like AHD, VNG, etc.
        
        let width = raw_image.width;
        let height = raw_image.height;
        let mut rgb_data = vec![0u16; width * height * 3];
        
        // Simple nearest-neighbor demosaicing for Bayer pattern
        // This assumes RGGB pattern - you'd need to detect the actual CFA pattern
        
        for y in 0..height {
            for x in 0..width {
                let src_idx = y * width + x;
                let dst_idx = (y * width + x) * 3;
                
                if src_idx >= data.len() || dst_idx + 2 >= rgb_data.len() {
                    continue;
                }
                
                // Determine pixel type based on position (RGGB pattern)
                let is_red_row = y % 2 == 0;
                let is_red_col = x % 2 == 0;
                
                let pixel_value = data[src_idx];
                
                match (is_red_row, is_red_col) {
                    (true, true) => {
                        // Red pixel
                        rgb_data[dst_idx] = pixel_value;     // R
                        rgb_data[dst_idx + 1] = self.interpolate_green(data, x, y, width, height); // G
                        rgb_data[dst_idx + 2] = self.interpolate_blue(data, x, y, width, height);  // B
                    }
                    (true, false) => {
                        // Green pixel (red row)
                        rgb_data[dst_idx] = self.interpolate_red(data, x, y, width, height);      // R
                        rgb_data[dst_idx + 1] = pixel_value; // G
                        rgb_data[dst_idx + 2] = self.interpolate_blue(data, x, y, width, height); // B
                    }
                    (false, true) => {
                        // Green pixel (blue row)
                        rgb_data[dst_idx] = self.interpolate_red(data, x, y, width, height);       // R
                        rgb_data[dst_idx + 1] = pixel_value; // G
                        rgb_data[dst_idx + 2] = self.interpolate_blue(data, x, y, width, height);  // B
                    }
                    (false, false) => {
                        // Blue pixel
                        rgb_data[dst_idx] = self.interpolate_red(data, x, y, width, height);      // R
                        rgb_data[dst_idx + 1] = self.interpolate_green(data, x, y, width, height); // G
                        rgb_data[dst_idx + 2] = pixel_value; // B
                    }
                }
            }
        }
        
        Ok(rgb_data)
    }
    
    fn interpolate_green(&self, data: &[u16], x: usize, y: usize, width: usize, height: usize) -> u16 {
        let mut sum = 0u32;
        let mut count = 0u32;
        
        // Sample neighboring green pixels
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    
                    // Check if this position has green in RGGB pattern
                    if (ny % 2 == 0 && nx % 2 == 1) || (ny % 2 == 1 && nx % 2 == 0) {
                        let idx = ny * width + nx;
                        if idx < data.len() {
                            sum += data[idx] as u32;
                            count += 1;
                        }
                    }
                }
            }
        }
        
        if count > 0 { (sum / count) as u16 } else { 0 }
    }
    
    fn interpolate_red(&self, data: &[u16], x: usize, y: usize, width: usize, height: usize) -> u16 {
        let mut sum = 0u32;
        let mut count = 0u32;
        
        // Sample neighboring red pixels (top-left in RGGB)
        for dy in -2i32..=2i32 {
            for dx in -2i32..=2i32 {
                if dx == 0 && dy == 0 { continue; }
                
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    
                    // Check if this position has red in RGGB pattern
                    if ny % 2 == 0 && nx % 2 == 0 {
                        let idx = ny * width + nx;
                        if idx < data.len() {
                            sum += data[idx] as u32;
                            count += 1;
                        }
                    }
                }
            }
        }
        
        if count > 0 { (sum / count) as u16 } else { 0 }
    }
    
    fn interpolate_blue(&self, data: &[u16], x: usize, y: usize, width: usize, height: usize) -> u16 {
        let mut sum = 0u32;
        let mut count = 0u32;
        
        // Sample neighboring blue pixels (bottom-right in RGGB)
        for dy in -2i32..=2i32 {
            for dx in -2i32..=2i32 {
                if dx == 0 && dy == 0 { continue; }
                
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    
                    // Check if this position has blue in RGGB pattern
                    if ny % 2 == 1 && nx % 2 == 1 {
                        let idx = ny * width + nx;
                        if idx < data.len() {
                            sum += data[idx] as u32;
                            count += 1;
                        }
                    }
                }
            }
        }
        
        if count > 0 { (sum / count) as u16 } else { 0 }
    }
    
    fn rgb_to_rgba(&self, rgb_data: &[u16], width: u32, height: u32) -> Result<Vec<u8>, LoadError> {
        let pixel_count = (width * height) as usize;
        let expected_rgb_len = pixel_count * 3;
        
        if rgb_data.len() != expected_rgb_len {
            return Err(LoadError::InvalidData(
                format!("RGB data length mismatch: expected {}, got {}", expected_rgb_len, rgb_data.len())
            ));
        }
        
        let mut rgba_data = Vec::with_capacity(pixel_count * 4);
        
        for i in 0..pixel_count {
            let base_idx = i * 3;
            // Convert from u16 to u8 and add alpha channel
            rgba_data.push((rgb_data[base_idx] >> 8) as u8);         // R
            rgba_data.push((rgb_data[base_idx + 1] >> 8) as u8);     // G
            rgba_data.push((rgb_data[base_idx + 2] >> 8) as u8);     // B
            rgba_data.push(255);                                     // A (fully opaque)
        }
        
        Ok(rgba_data)
    }
    
    pub fn get_image_metadata<P: AsRef<Path>>(&self, path: P) -> Result<ImageMetadata, LoadError> {
        let path = path.as_ref();
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| LoadError::UnsupportedFormat("No extension".to_string()))?;
        
        if self.is_raw_format(&extension) {
            let raw_image = decode_file(path)
                .map_err(|e| LoadError::RawDecodeError(format!("Failed to decode RAW: {}", e)))?;
            
            Ok(ImageMetadata {
                width: raw_image.width as u32,
                height: raw_image.height as u32,
                is_raw: true,
                color_space: raw_image.color_space.clone().unwrap_or_else(|| "Unknown".to_string()),
                white_balance: raw_image.wb_coeffs.clone(),
                iso: raw_image.iso,
                exposure_time: raw_image.exposure_time,
                aperture: raw_image.aperture,
            })
        } else {
            // For standard images, we'd need to use image crate's metadata
            let img = image::open(path)
                .map_err(|e| LoadError::ImageOpenError(format!("Failed to open image: {}", e)))?;
            
            Ok(ImageMetadata {
                width: img.width(),
                height: img.height(),
                is_raw: false,
                color_space: "sRGB".to_string(),
                white_balance: None,
                iso: None,
                exposure_time: None,
                aperture: None,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageMetadata {
    pub width: u32,
    pub height: u32,
    pub is_raw: bool,
    pub color_space: String,
    pub white_balance: Option<Vec<f32>>,
    pub iso: Option<u16>,
    pub exposure_time: Option<f32>,
    pub aperture: Option<f32>,
}