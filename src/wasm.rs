//! WASM bindings for browser-based PILATUS4 data exploration
//! 
//! Exposes the Rust library to JavaScript via wasm-bindgen.
//! All data passed as JSON for seamless JS interoperability.

use wasm_bindgen::prelude::*;
use serde_json::json;
use ndarray::{Array2, ArrayView2};
use std::collections::HashMap;

use crate::io::*;
use crate::processing::*;

/// Main WASM interface for data processing
#[wasm_bindgen]
pub struct DataExplorer {
    detector: Option<DetectorConfig>,
    calibration: Option<AzimuthalIntegrator>,
    bright_field: Option<Array2<f32>>,
    mask: Option<Array2<bool>>,
    image: Option<Array2<u32>>,
}

#[wasm_bindgen]
impl DataExplorer {
    /// Create a new data explorer instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> DataExplorer {
        DataExplorer {
            detector: None,
            calibration: None,
            bright_field: None,
            mask: None,
            image: None,
        }
    }

    /// Load PONI calibration from text content
    pub fn load_poni(&mut self, poni_content: &str) -> Result<String, String> {
        match parse_poni_text(poni_content) {
            Ok(ai) => {
                self.calibration = Some(ai.clone());
                Ok(json!({
                    "status": "success",
                    "detector": ai.detector,
                    "distance": ai.distance,
                    "wavelength": ai.wavelength,
                    "poni1": ai.poni1,
                    "poni2": ai.poni2,
                }).to_string())
            }
            Err(e) => Err(format!("PONI parsing error: {}", e)),
        }
    }

    /// Load bright field correction from base64-encoded .npy
    pub fn load_bright_field(&mut self, npy_base64: &str) -> Result<String, String> {
        let bytes = base64_to_bytes(npy_base64)
            .map_err(|e| format!("Base64 decode error: {}", e))?;
        
        match load_bright_field_bytes(&bytes) {
            Ok(ff) => {
                let (rows, cols) = ff.dim();
                let min = ff.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max = ff.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                
                self.bright_field = Some(ff);
                Ok(json!({
                    "status": "success",
                    "shape": [rows, cols],
                    "min": min,
                    "max": max,
                }).to_string())
            }
            Err(e) => Err(format!("Bright field load error: {}", e)),
        }
    }

    /// Load pixel mask from base64-encoded .edf
    pub fn load_mask(&mut self, edf_base64: &str) -> Result<String, String> {
        let bytes = base64_to_bytes(edf_base64)
            .map_err(|e| format!("Base64 decode error: {}", e))?;
        
        match load_mask_bytes(&bytes) {
            Ok(mask) => {
                let (rows, cols) = mask.dim();
                let masked = mask.iter().filter(|&&x| x).count();
                let total = rows * cols;
                let pct = 100.0 * masked as f64 / total as f64;
                
                self.mask = Some(mask);
                Ok(json!({
                    "status": "success",
                    "shape": [rows, cols],
                    "masked_pixels": masked,
                    "total_pixels": total,
                    "percentage": pct,
                }).to_string())
            }
            Err(e) => Err(format!("Mask load error: {}", e)),
        }
    }

    /// Load sample image from base64-encoded .tiff
    pub fn load_image(&mut self, tiff_base64: &str) -> Result<String, String> {
        let bytes = base64_to_bytes(tiff_base64)
            .map_err(|e| format!("Base64 decode error: {}", e))?;
        
        match load_tiff_bytes(&bytes) {
            Ok(img) => {
                let (rows, cols) = img.dim();
                let min = img.iter().fold(u32::MAX, |a, &b| a.min(b));
                let max = img.iter().fold(0u32, |a, &b| a.max(b));
                let mean = img.iter().sum::<u32>() as f64 / (rows * cols) as f64;
                
                self.image = Some(img);
                Ok(json!({
                    "status": "success",
                    "shape": [rows, cols],
                    "min": min,
                    "max": max,
                    "mean": mean,
                }).to_string())
            }
            Err(e) => Err(format!("Image load error: {}", e)),
        }
    }

    /// Process loaded data: apply corrections and integration
    pub fn process(
        &self,
        tth_min: f64,
        tth_max: f64,
        n_bins: usize,
    ) -> Result<String, String> {
        // Validate all required data is loaded
        let calibration = self.calibration.as_ref()
            .ok_or_else(|| "PONI calibration not loaded".to_string())?;
        let bright_field = self.bright_field.as_ref()
            .ok_or_else(|| "Bright field not loaded".to_string())?;
        let image = self.image.as_ref()
            .ok_or_else(|| "Image not loaded".to_string())?;

        // Step 1: Apply flat field correction
        let image_view: ArrayView2<u32> = image.view();
        let corrected = apply_flatfield(&image_view, &bright_field.view())
            .map_err(|e| format!("Flatfield error: {}", e))?;

        // Step 2: Create integration geometry from calibration
        let geom = IntegrationGeometry::from(calibration);

        // Step 3: Azimuthal integration
        // Chi range: full rotation (0° to 360°), chi bins: same as tth bins
        let result = azimuthal_integrate(
            &corrected.view(),
            &geom,
            tth_min,
            tth_max,
            n_bins,
            0.0,           // chi_min: 0°
            360.0,         // chi_max: 360°
            1,             // chi_bins: integrate over all azimuth
        )
        .map_err(|e| format!("Integration error: {}", e))?;

        // Return results as JSON
        // Collapse 2D result (n_bins × 1) to 1D
        let intensity_1d: Vec<f64> = result.intensity.iter()
            .step_by(result.intensity.ncols())
            .copied()
            .collect();
        
        let counts_1d: Vec<u32> = result.counts.iter()
            .step_by(result.counts.ncols())
            .copied()
            .collect();
        
        // Calculate standard errors from counts (Poisson statistics)
        let error_1d: Vec<f64> = counts_1d.iter()
            .map(|&c| (c as f64).sqrt())
            .collect();

        let result_json = json!({
            "status": "success",
            "tth_min": tth_min,
            "tth_max": tth_max,
            "n_bins": n_bins,
            "intensity": intensity_1d,
            "error": error_1d,
            "counts": counts_1d,
        });

        Ok(result_json.to_string())
    }

    /// Get detector info
    pub fn detector_info(&self) -> String {
        let config = DetectorConfig::default();
        json!({
            "name": config.name,
            "n_rows": config.n_rows,
            "n_cols": config.n_cols,
            "pixel_size_1": config.pixel_size_1,
            "pixel_size_2": config.pixel_size_2,
        }).to_string()
    }

    /// Get current data status
    pub fn status(&self) -> String {
        json!({
            "detector": self.detector.is_some(),
            "calibration": self.calibration.is_some(),
            "bright_field": self.bright_field.is_some(),
            "mask": self.mask.is_some(),
            "image": self.image.is_some(),
        }).to_string()
    }
}

// ============================================================================
// INTERNAL HELPERS - NOT EXPOSED TO JS
// ============================================================================

/// Parse PONI text content directly (no file system)
fn parse_poni_text(content: &str) -> Result<AzimuthalIntegrator, String> {
    let mut detector = String::new();
    let mut distance = 0.0;
    let mut wavelength = 0.0;
    let mut poni1 = 0.0;
    let mut poni2 = 0.0;
    let mut rot1 = 0.0;
    let mut rot2 = 0.0;
    let mut rot3 = 0.0;
    let mut detector_config = HashMap::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim();
            let val = val.trim();
            
            match key {
                "Distance" => {
                    let dist: f64 = val.split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| "Invalid distance".to_string())?;
                    distance = if val.contains("mm") { dist / 1000.0 } else { dist };
                }
                "Wavelength" => {
                    wavelength = val.parse()
                        .map_err(|_| "Invalid wavelength".to_string())?;
                }
                "Poni1" => {
                    poni1 = val.split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| "Invalid poni1".to_string())?;
                }
                "Poni2" => {
                    poni2 = val.split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| "Invalid poni2".to_string())?;
                }
                "Rot1" => {
                    rot1 = val.split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                }
                "Rot2" => {
                    rot2 = val.split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                }
                "Rot3" => {
                    rot3 = val.split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                }
                "Detector" => {
                    detector = val.to_string();
                }
                _ => {}
            }
        }
    }

    Ok(AzimuthalIntegrator {
        detector,
        detector_config,
        distance,
        wavelength,
        poni1,
        poni2,
        rot1,
        rot2,
        rot3,
        pixel_size_1: 150.0e-6,  // Default: 150 µm
        pixel_size_2: 150.0e-6,
        poni_version: "2.1".to_string(),
    })
}

/// Load bright field from binary bytes
fn load_bright_field_bytes(_bytes: &[u8]) -> Result<Array2<f32>, String> {
    // In production WASM, we'd need to convert bytes to actual .npy format
    // For now, return a placeholder
    Err("Binary loading in WASM requires custom implementation".to_string())
}

/// Load mask from binary bytes
fn load_mask_bytes(_bytes: &[u8]) -> Result<Array2<bool>, String> {
    // In production WASM, convert bytes to Array2<bool>
    Err("Binary loading in WASM requires custom implementation".to_string())
}

/// Load TIFF from binary bytes
fn load_tiff_bytes(_bytes: &[u8]) -> Result<Array2<u32>, String> {
    // In production WASM, use the tiff crate to parse bytes
    Err("Binary loading in WASM requires custom implementation".to_string())
}

/// Decode base64 string to bytes
fn base64_to_bytes(_base64_str: &str) -> Result<Vec<u8>, String> {
    // Use the base64 crate to decode
    // For now, return placeholder error
    Err("Base64 decoding requires external crate".to_string())
}
