// src/adjustment_state.rs
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct AdjustmentState {
    // Basic adjustments
    pub exposure: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    
    // Color adjustments
    pub saturation: f32,
    pub vibrance: f32,
    
    // White balance
    pub temperature: f32,
    pub tint: f32,
    
    // Advanced adjustments
    pub clarity: f32,
    pub dehaze: f32,
    pub noise_reduction: f32,
    pub sharpening: f32,
    
    // Tone curve points (for future curve implementation)
    pub tone_curve: ToneCurve,
    
    // Color grading
    pub color_grading: ColorGrading,
    
    // Lens corrections
    pub lens_corrections: LensCorrections,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToneCurve {
    pub points: Vec<CurvePoint>,
    pub curve_type: CurveType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurvePoint {
    pub input: f32,   // 0.0 to 1.0
    pub output: f32,  // 0.0 to 1.0
}

#[derive(Clone, Debug, PartialEq)]
pub enum CurveType {
    Linear,
    Smooth,
    Sharp,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorGrading {
    pub shadows_hue: f32,
    pub shadows_saturation: f32,
    pub shadows_luminance: f32,
    pub midtones_hue: f32,
    pub midtones_saturation: f32,
    pub midtones_luminance: f32,
    pub highlights_hue: f32,
    pub highlights_saturation: f32,
    pub highlights_luminance: f32,
    pub global_hue: f32,
    pub global_saturation: f32,
    pub global_luminance: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LensCorrections {
    pub chromatic_aberration: f32,
    pub vignetting: f32,
    pub distortion: f32,
    pub lens_profile_enabled: bool,
    pub lens_profile_name: Option<String>,
}

impl Default for AdjustmentState {
    fn default() -> Self {
        Self {
            // Basic adjustments - neutral values
            exposure: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            
            // Color adjustments - neutral values
            saturation: 0.0,
            vibrance: 0.0,
            
            // White balance - neutral values
            temperature: 0.0,
            tint: 0.0,
            
            // Advanced adjustments - neutral values
            clarity: 0.0,
            dehaze: 0.0,
            noise_reduction: 0.0,
            sharpening: 0.0,
            
            // Default tone curve (linear)
            tone_curve: ToneCurve::default(),
            
            // Default color grading (neutral)
            color_grading: ColorGrading::default(),
            
            // Default lens corrections (disabled)
            lens_corrections: LensCorrections::default(),
        }
    }
}

impl Default for ToneCurve {
    fn default() -> Self {
        Self {
            points: vec![
                CurvePoint { input: 0.0, output: 0.0 },   // Black point
                CurvePoint { input: 1.0, output: 1.0 },   // White point
            ],
            curve_type: CurveType::Linear,
        }
    }
}

impl Default for ColorGrading {
    fn default() -> Self {
        Self {
            shadows_hue: 0.0,
            shadows_saturation: 0.0,
            shadows_luminance: 0.0,
            midtones_hue: 0.0,
            midtones_saturation: 0.0,
            midtones_luminance: 0.0,
            highlights_hue: 0.0,
            highlights_saturation: 0.0,
            highlights_luminance: 0.0,
            global_hue: 0.0,
            global_saturation: 0.0,
            global_luminance: 0.0,
        }
    }
}

impl Default for LensCorrections {
    fn default() -> Self {
        Self {
            chromatic_aberration: 0.0,
            vignetting: 0.0,
            distortion: 0.0,
            lens_profile_enabled: false,
            lens_profile_name: None,
        }
    }
}

impl AdjustmentState {
    /// Reset all adjustments to their default (neutral) values
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    
    /// Check if any adjustments have been made from the default values
    pub fn has_changes(&self) -> bool {
        self.exposure.abs() > f32::EPSILON ||
        self.contrast.abs() > f32::EPSILON ||
        self.highlights.abs() > f32::EPSILON ||
        self.shadows.abs() > f32::EPSILON ||
        self.whites.abs() > f32::EPSILON ||
        self.blacks.abs() > f32::EPSILON ||
        self.saturation.abs() > f32::EPSILON ||
        self.vibrance.abs() > f32::EPSILON ||
        self.temperature.abs() > f32::EPSILON ||
        self.tint.abs() > f32::EPSILON ||
        self.clarity.abs() > f32::EPSILON ||
        self.dehaze.abs() > f32::EPSILON ||
        self.noise_reduction.abs() > f32::EPSILON ||
        self.sharpening.abs() > f32::EPSILON ||
        self.tone_curve.has_changes() ||
        self.color_grading.has_changes() ||
        self.lens_corrections.has_changes()
    }
    
    /// Get a summary of current adjustments
    pub fn get_adjustment_summary(&self) -> Vec<String> {
        let mut summary = Vec::new();
        
        if self.exposure.abs() > f32::EPSILON {
            summary.push(format!("Exposure: {:.2}", self.exposure));
        }
        if self.contrast.abs() > f32::EPSILON {
            summary.push(format!("Contrast: {:.0}", self.contrast));
        }
        if self.highlights.abs() > f32::EPSILON {
            summary.push(format!("Highlights: {:.0}", self.highlights));
        }
        if self.shadows.abs() > f32::EPSILON {
            summary.push(format!("Shadows: {:.0}", self.shadows));
        }
        if self.saturation.abs() > f32::EPSILON {
            summary.push(format!("Saturation: {:.0}", self.saturation));
        }
        if self.vibrance.abs() > f32::EPSILON {
            summary.push(format!("Vibrance: {:.0}", self.vibrance));
        }
        if self.temperature.abs() > f32::EPSILON {
            summary.push(format!("Temperature: {:.0}K", self.temperature));
        }
        
        if summary.is_empty() {
            summary.push("No adjustments".to_string());
        }
        
        summary
    }
    
    /// Create a preset from current settings
    pub fn create_preset(&self, name: String) -> AdjustmentPreset {
        AdjustmentPreset {
            name,
            adjustments: self.clone(),
            created_at: std::time::SystemTime::now(),
        }
    }
    
    /// Apply a preset to current settings
    pub fn apply_preset(&mut self, preset: &AdjustmentPreset) {
        *self = preset.adjustments.clone();
    }
    
    /// Get adjustment value by name (for generic UI controls)
    pub fn get_adjustment(&self, name: &str) -> Option<f32> {
        match name {
            "exposure" => Some(self.exposure),
            "contrast" => Some(self.contrast),
            "highlights" => Some(self.highlights),
            "shadows" => Some(self.shadows),
            "whites" => Some(self.whites),
            "blacks" => Some(self.blacks),
            "saturation" => Some(self.saturation),
            "vibrance" => Some(self.vibrance),
            "temperature" => Some(self.temperature),
            "tint" => Some(self.tint),
            "clarity" => Some(self.clarity),
            "dehaze" => Some(self.dehaze),
            "noise_reduction" => Some(self.noise_reduction),
            "sharpening" => Some(self.sharpening),
            _ => None,
        }
    }
    
    /// Set adjustment value by name (for generic UI controls)
    pub fn set_adjustment(&mut self, name: &str, value: f32) -> bool {
        match name {
            "exposure" => { self.exposure = value; true }
            "contrast" => { self.contrast = value; true }
            "highlights" => { self.highlights = value; true }
            "shadows" => { self.shadows = value; true }
            "whites" => { self.whites = value; true }
            "blacks" => { self.blacks = value; true }
            "saturation" => { self.saturation = value; true }
            "vibrance" => { self.vibrance = value; true }
            "temperature" => { self.temperature = value; true }
            "tint" => { self.tint = value; true }
            "clarity" => { self.clarity = value; true }
            "dehaze" => { self.dehaze = value; true }
            "noise_reduction" => { self.noise_reduction = value; true }
            "sharpening" => { self.sharpening = value; true }
            _ => false,
        }
    }
    
    /// Validate adjustment values are within acceptable ranges
    pub fn validate(&mut self) {
        self.exposure = self.exposure.clamp(-5.0, 5.0);
        self.contrast = self.contrast.clamp(-100.0, 100.0);
        self.highlights = self.highlights.clamp(-100.0, 100.0);
        self.shadows = self.shadows.clamp(-100.0, 100.0);
        self.whites = self.whites.clamp(-100.0, 100.0);
        self.blacks = self.blacks.clamp(-100.0, 100.0);
        self.saturation = self.saturation.clamp(-100.0, 100.0);
        self.vibrance = self.vibrance.clamp(-100.0, 100.0);
        self.temperature = self.temperature.clamp(-100.0, 100.0);
        self.tint = self.tint.clamp(-100.0, 100.0);
        self.clarity = self.clarity.clamp(-100.0, 100.0);
        self.dehaze = self.dehaze.clamp(-100.0, 100.0);
        self.noise_reduction = self.noise_reduction.clamp(0.0, 100.0);
        self.sharpening = self.sharpening.clamp(0.0, 100.0);
    }
}

impl ToneCurve {
    /// Check if tone curve has been modified from default
    pub fn has_changes(&self) -> bool {
        self.points.len() != 2 ||
        self.points[0] != CurvePoint { input: 0.0, output: 0.0 } ||
        self.points[1] != CurvePoint { input: 1.0, output: 1.0 } ||
        self.curve_type != CurveType::Linear
    }
    
    /// Add a curve point
    pub fn add_point(&mut self, input: f32, output: f32) {
        let point = CurvePoint {
            input: input.clamp(0.0, 1.0),
            output: output.clamp(0.0, 1.0),
        };
        
        // Insert point in sorted order
        let insert_index = self.points
            .binary_search_by(|p| p.input.partial_cmp(&point.input).unwrap())
            .unwrap_or_else(|i| i);
        
        self.points.insert(insert_index, point);
    }
    
    /// Remove a curve point by index
    pub fn remove_point(&mut self, index: usize) -> bool {
        if index > 0 && index < self.points.len() - 1 { // Don't remove first or last point
            self.points.remove(index);
            true
        } else {
            false
        }
    }
    
    /// Evaluate the curve at a given input value
    pub fn evaluate(&self, input: f32) -> f32 {
        let input = input.clamp(0.0, 1.0);
        
        // Find the two points to interpolate between
        for i in 0..self.points.len() - 1 {
            let p1 = &self.points[i];
            let p2 = &self.points[i + 1];
            
            if input >= p1.input && input <= p2.input {
                if (p2.input - p1.input).abs() < f32::EPSILON {
                    return p1.output;
                }
                
                let t = (input - p1.input) / (p2.input - p1.input);
                
                return match self.curve_type {
                    CurveType::Linear => p1.output + t * (p2.output - p1.output),
                    CurveType::Smooth => {
                        // Smooth interpolation using cubic bezier-like curve
                        let t2 = t * t;
                        let t3 = t2 * t;
                        p1.output * (1.0 - 3.0 * t2 + 2.0 * t3) + p2.output * (3.0 * t2 - 2.0 * t3)
                    }
                    CurveType::Sharp => {
                        // Sharp transition
                        if t < 0.5 { p1.output } else { p2.output }
                    }
                };
            }
        }
        
        // Should not reach here, but return input as fallback
        input
    }
    
    /// Reset to linear curve
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl ColorGrading {
    /// Check if color grading has been modified from default
    pub fn has_changes(&self) -> bool {
        self.shadows_hue.abs() > f32::EPSILON ||
        self.shadows_saturation.abs() > f32::EPSILON ||
        self.shadows_luminance.abs() > f32::EPSILON ||
        self.midtones_hue.abs() > f32::EPSILON ||
        self.midtones_saturation.abs() > f32::EPSILON ||
        self.midtones_luminance.abs() > f32::EPSILON ||
        self.highlights_hue.abs() > f32::EPSILON ||
        self.highlights_saturation.abs() > f32::EPSILON ||
        self.highlights_luminance.abs() > f32::EPSILON ||
        self.global_hue.abs() > f32::EPSILON ||
        self.global_saturation.abs() > f32::EPSILON ||
        self.global_luminance.abs() > f32::EPSILON
    }
    
    /// Reset all color grading to neutral
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl LensCorrections {
    /// Check if lens corrections have been modified from default
    pub fn has_changes(&self) -> bool {
        self.chromatic_aberration.abs() > f32::EPSILON ||
        self.vignetting.abs() > f32::EPSILON ||
        self.distortion.abs() > f32::EPSILON ||
        self.lens_profile_enabled
    }
    
    /// Reset all lens corrections
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Debug)]
pub struct AdjustmentPreset {
    pub name: String,
    pub adjustments: AdjustmentState,
    pub created_at: std::time::SystemTime,
}

impl AdjustmentPreset {
    pub fn new(name: String, adjustments: AdjustmentState) -> Self {
        Self {
            name,
            adjustments,
            created_at: std::time::SystemTime::now(),
        }
    }
}

/// Preset manager for saving and loading adjustment presets
pub struct PresetManager {
    presets: HashMap<String, AdjustmentPreset>,
}

impl PresetManager {
    pub fn new() -> Self {
        Self {
            presets: HashMap::new(),
        }
    }
    
    /// Save a preset
    pub fn save_preset(&mut self, preset: AdjustmentPreset) {
        self.presets.insert(preset.name.clone(), preset);
    }
    
    /// Load a preset by name
    pub fn load_preset(&self, name: &str) -> Option<&AdjustmentPreset> {
        self.presets.get(name)
    }
    
    /// Get all preset names
    pub fn get_preset_names(&self) -> Vec<String> {
        self.presets.keys().cloned().collect()
    }
    
    /// Delete a preset
    pub fn delete_preset(&mut self, name: &str) -> bool {
        self.presets.remove(name).is_some()
    }
    
    /// Get all presets
    pub fn get_all_presets(&self) -> Vec<&AdjustmentPreset> {
        self.presets.values().collect()
    }
    
    /// Create some default presets
    pub fn create_default_presets(&mut self) {
        // High contrast preset
        let mut high_contrast = AdjustmentState::default();
        high_contrast.contrast = 50.0;
        high_contrast.clarity = 30.0;
        high_contrast.whites = 20.0;
        high_contrast.blacks = -20.0;
        self.save_preset(AdjustmentPreset::new("High Contrast".to_string(), high_contrast));
        
        // Warm and bright preset  
        let mut warm_bright = AdjustmentState::default();
        warm_bright.exposure = 0.5;
        warm_bright.temperature = 200.0;
        warm_bright.vibrance = 25.0;
        warm_bright.shadows = 30.0;
        self.save_preset(AdjustmentPreset::new("Warm & Bright".to_string(), warm_bright));
        
        // Cool and moody preset
        let mut cool_moody = AdjustmentState::default();
        cool_moody.exposure = -0.3;
        cool_moody.temperature = -300.0;
        cool_moody.highlights = -50.0;
        cool_moody.contrast = 20.0;
        cool_moody.clarity = 15.0;
        self.save_preset(AdjustmentPreset::new("Cool & Moody".to_string(), cool_moody));
        
        // Portrait preset
        let mut portrait = AdjustmentState::default();
        portrait.clarity = -20.0; // Softer skin
        portrait.vibrance = 15.0;
        portrait.shadows = 20.0;
        portrait.noise_reduction = 25.0;
        self.save_preset(AdjustmentPreset::new("Portrait".to_string(), portrait));
        
        // Landscape preset
        let mut landscape = AdjustmentState::default();
        landscape.clarity = 40.0;
        landscape.vibrance = 30.0;
        landscape.contrast = 25.0;
        landscape.dehaze = 20.0;
        landscape.sharpening = 30.0;
        self.save_preset(AdjustmentPreset::new("Landscape".to_string(), landscape));
    }
}