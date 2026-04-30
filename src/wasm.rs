//! WASM bindings for browser-based PILATUS4 data exploration
//! 
//! Exposes the Rust library to JavaScript via wasm-bindgen.
//! All data passed as JSON for seamless JS interoperability.

use wasm_bindgen::prelude::*;
use serde_json::json;
use ndarray::{Array2, ArrayView2};
use std::collections::HashMap;
use base64::{engine::general_purpose, Engine as _};

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
        // Debug: log the input
        let b64_len = edf_base64.len();
        
        let bytes = base64_to_bytes(edf_base64)
            .map_err(|e| format!("Base64 decode error (input {} chars): {}", b64_len, e))?;
        
        let bytes_len = bytes.len();
        
        // Debug: check first bytes to verify it's a valid EDF
        let is_valid_edf = bytes.len() > 10 && &bytes[0..1] == b"{";
        
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
            Err(e) => Err(format!("Mask load error (bytes: {}, b64_len: {}, valid_edf: {}): {}", 
                bytes_len, b64_len, is_valid_edf, e)),
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
        // Validate required data is loaded
        let calibration = self.calibration.as_ref()
            .ok_or_else(|| "PONI calibration not loaded".to_string())?;
        let image = self.image.as_ref()
            .ok_or_else(|| "Image not loaded".to_string())?;

        // Step 1: Apply flat field correction (optional)
        let image_view: ArrayView2<u32> = image.view();
        let corrected = if let Some(bf) = &self.bright_field {
            apply_flatfield(&image_view, &bf.view())
                .map_err(|e| format!("Flatfield error: {}", e))?
        } else {
            // Convert image to f32 without flatfield correction
            image_view.mapv(|x| x as f32)
        };

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

    /// Get detector image as base64-encoded raw pixel data (for visualization)
    /// Returns PNG representation for easier display
    pub fn get_image_data(&self) -> Result<String, String> {
        let image = self.image.as_ref()
            .ok_or_else(|| "Image not loaded".to_string())?;
        
        let (n_rows, n_cols) = image.dim();
        
        // Find min/max for normalization
        let min_val = image.iter().copied().fold(u32::MAX, u32::min);
        let max_val = image.iter().copied().fold(0u32, u32::max);
        let range = (max_val - min_val).max(1);
        
        // Create RGBA8 image buffer (each pixel as 4 bytes: R, G, B, A)
        let mut rgb_data = Vec::with_capacity(n_rows * n_cols * 4);
        
        for &pixel in image.iter() {
            // Normalize to 0-255 and apply viridis-like colormap
            let norm = ((pixel - min_val) as f32) / (range as f32);
            let norm = norm.max(0.0).min(1.0);
            
            // Simple viridis approximation
            let (r, g, b) = if norm < 0.25 {
                // Dark purple to blue
                let t = norm / 0.25;
                (32.0 + t * 64.0, 0.0, 32.0 + t * 64.0)
            } else if norm < 0.5 {
                // Blue to cyan
                let t = (norm - 0.25) / 0.25;
                (0.0 + t * 255.0, 64.0 + t * 128.0, 96.0 + t * 128.0)
            } else if norm < 0.75 {
                // Cyan to green
                let t = (norm - 0.5) / 0.25;
                (255.0 - t * 255.0, 192.0 + t * 63.0, 0.0 + t * 0.0)
            } else {
                // Green to yellow
                let t = (norm - 0.75) / 0.25;
                (255.0, 255.0 - t * 64.0, t * 255.0)
            };
            
            rgb_data.push((r as u8).saturating_add(0));
            rgb_data.push((g as u8).saturating_add(0));
            rgb_data.push((b as u8).saturating_add(0));
            rgb_data.push(255u8); // Alpha
        }
        
        // Return as base64
        let b64 = general_purpose::STANDARD.encode(&rgb_data);
        
        Ok(json!({
            "status": "success",
            "width": n_cols,
            "height": n_rows,
            "min_val": min_val,
            "max_val": max_val,
            "rgba_base64": b64,
        }).to_string())
    }

    /// Compute and return LUT (Look-Up Table) geometry for debugging
    /// Returns 2θ and χ values for all detector pixels
    pub fn get_lut(&self) -> Result<String, String> {
        let calibration = self.calibration.as_ref()
            .ok_or_else(|| "PONI calibration not loaded".to_string())?;
        let image = self.image.as_ref()
            .ok_or_else(|| "Image not loaded".to_string())?;

        let geom = IntegrationGeometry::from(calibration);
        let (n_rows, n_cols) = image.dim();
        
        // Precompute rotation matrix
        let c1 = geom.rot1.cos();
        let s1 = geom.rot1.sin();
        let c2 = geom.rot2.cos();
        let s2 = geom.rot2.sin();
        let c3 = geom.rot3.cos();
        let s3 = geom.rot3.sin();
        
        let r11 = c2 * c3;
        let r12 = s1 * s2 * c3 - c1 * s3;
        let r21 = c2 * s3;
        let r22 = s1 * s2 * s3 + c1 * c3;
        let r31 = -s2;
        let r32 = s1 * c2;
        
        // Collect sample of LUT values (every Nth pixel to avoid huge output)
        let mut lut_samples = Vec::new();
        let step = (n_rows / 50).max(1); // Sample ~50 rows
        
        for i in (0..n_rows).step_by(step) {
            for j in (0..n_cols).step_by(step) {
                let y_pixel_m = i as f64 * geom.pixel_size_1 - geom.poni1;
                let x_pixel_m = j as f64 * geom.pixel_size_2 - geom.poni2;
                
                let x_rot = r11 * x_pixel_m + r12 * y_pixel_m;
                let y_rot = r21 * x_pixel_m + r22 * y_pixel_m;
                let z_component = r31 * x_pixel_m + r32 * y_pixel_m;
                let z_pos = geom.distance + z_component;
                
                let r_transverse = (x_rot * x_rot + y_rot * y_rot).sqrt();
                let two_theta_rad = 2.0 * (r_transverse / z_pos).atan();
                let two_theta_deg = two_theta_rad.to_degrees();
                
                let chi_rad = y_rot.atan2(x_rot);
                let chi_deg = chi_rad.to_degrees();
                
                lut_samples.push(json!({
                    "pixel_i": i,
                    "pixel_j": j,
                    "two_theta_deg": two_theta_deg,
                    "chi_deg": chi_deg,
                }));
            }
        }
        
        Ok(json!({
            "status": "success",
            "detector_shape": [n_rows, n_cols],
            "poni1_m": geom.poni1,
            "poni2_m": geom.poni2,
            "distance_m": geom.distance,
            "wavelength_m": geom.wavelength,
            "lut_samples": lut_samples,
        }).to_string())
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
    let detector_config = HashMap::new();
    
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
fn load_bright_field_bytes(bytes: &[u8]) -> Result<Array2<f32>, String> {
    load_bright_field_from_bytes(bytes)
        .map_err(|e| format!("Bright field load error: {}", e))
}

/// Load mask from binary bytes
fn load_mask_bytes(bytes: &[u8]) -> Result<Array2<bool>, String> {
    load_mask_from_bytes(bytes)
        .map_err(|e| format!("Mask load error: {}", e))
}

/// Load TIFF from binary bytes
fn load_tiff_bytes(bytes: &[u8]) -> Result<Array2<u32>, String> {
    load_tiff_from_bytes(bytes)
        .map_err(|e| format!("TIFF load error: {}", e))
}

/// Decode base64 string to bytes
fn base64_to_bytes(base64_str: &str) -> Result<Vec<u8>, String> {
    general_purpose::STANDARD
        .decode(base64_str.trim())
        .map_err(|e| format!("Base64 decode error: {}", e))
}
