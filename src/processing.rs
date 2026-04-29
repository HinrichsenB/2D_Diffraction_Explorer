//! Data processing for PILATUS4 detector data
//! 
//! Implements:
//! - Flatfield correction
//! - Fractile filtering (clipping at low/high values)
//! - Azimuthal integration (detector → reciprocal space)

use ndarray::{Array1, Array2, ArrayView2};
use std::f64::consts::PI;
use thiserror::Error;

use crate::io::AzimuthalIntegrator;

#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Shape mismatch: {0}")]
    ShapeMismatch(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Integration error: {0}")]
    IntegrationError(String),
}

pub type ProcessingResult<T> = Result<T, ProcessingError>;

/// Apply flatfield correction to raw detector data
/// 
/// Corrects pixel-by-pixel sensitivity variations:
/// corrected = raw / flatfield
pub fn apply_flatfield(
    image: &ArrayView2<u32>,
    flatfield: &ArrayView2<f32>,
) -> ProcessingResult<Array2<f32>> {
    if image.dim() != flatfield.dim() {
        return Err(ProcessingError::ShapeMismatch(
            format!("Image shape {:?} doesn't match flatfield shape {:?}", 
                    image.dim(), flatfield.dim())
        ));
    }
    
    let mut corrected = Array2::zeros(image.dim());
    
    for ((i, j), &pixel_value) in image.indexed_iter() {
        let ff_value = flatfield[[i, j]];
        if ff_value > 0.0 {
            corrected[[i, j]] = pixel_value as f32 / ff_value;
        } else {
            corrected[[i, j]] = 0.0;
        }
    }
    
    Ok(corrected)
}

/// Apply fractile (percentile) filtering
/// 
/// Clips values at specified low and high percentiles
/// Values below low_percentile → set to 0
/// Values above high_percentile → set to high_percentile value
pub fn fractile_filter(
    data: &ArrayView2<f32>,
    low_percentile: f32,
    high_percentile: f32,
) -> ProcessingResult<(Array2<f32>, Array2<bool>, Array2<bool>)> {
    if !(0.0..=100.0).contains(&low_percentile) || !(0.0..=100.0).contains(&high_percentile) {
        return Err(ProcessingError::InvalidParameter(
            format!("Percentiles must be in [0,100], got ({}, {})", 
                    low_percentile, high_percentile)
        ));
    }
    
    if low_percentile > high_percentile {
        return Err(ProcessingError::InvalidParameter(
            "Low percentile must be <= high percentile".to_string()
        ));
    }
    
    // Collect all values and sort to find percentiles
    let mut sorted: Vec<f32> = data.iter()
        .filter(|&&x| !x.is_nan() && !x.is_infinite())
        .copied()
        .collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let low_idx = ((low_percentile / 100.0) * sorted.len() as f32) as usize;
    let high_idx = ((high_percentile / 100.0) * sorted.len() as f32) as usize;
    
    let low_value = sorted.get(low_idx).copied().unwrap_or(0.0);
    let high_value = sorted.get(high_idx.saturating_sub(1)).copied().unwrap_or(1.0);
    
    let mut filtered = Array2::zeros(data.dim());
    let mut low_mask = Array2::from_elem(data.dim(), false);
    let mut high_mask = Array2::from_elem(data.dim(), false);
    
    for ((i, j), &value) in data.indexed_iter() {
        if value < low_value {
            filtered[[i, j]] = 0.0;
            low_mask[[i, j]] = true;
        } else if value > high_value {
            filtered[[i, j]] = high_value;
            high_mask[[i, j]] = true;
        } else {
            filtered[[i, j]] = value;
        }
    }
    
    Ok((filtered, low_mask, high_mask))
}

/// Geometry for azimuthal integration
pub struct IntegrationGeometry {
    /// Distance from sample to detector (meters)
    pub distance: f64,
    
    /// Vertical offset of beam center (meters)
    pub poni1: f64,
    
    /// Horizontal offset of beam center (meters)
    pub poni2: f64,
    
    /// Rotation angle 1 (radians)
    pub rot1: f64,
    
    /// Rotation angle 2 (radians)
    pub rot2: f64,
    
    /// Rotation angle 3 (radians)
    pub rot3: f64,
    
    /// Wavelength (meters)
    pub wavelength: f64,
    
    /// Pixel size in vertical direction (meters)
    pub pixel_size_1: f64,
    
    /// Pixel size in horizontal direction (meters)
    pub pixel_size_2: f64,
}

impl From<&AzimuthalIntegrator> for IntegrationGeometry {
    fn from(ai: &AzimuthalIntegrator) -> Self {
        Self {
            distance: ai.distance,
            poni1: ai.poni1,
            poni2: ai.poni2,
            rot1: ai.rot1,
            rot2: ai.rot2,
            rot3: ai.rot3,
            wavelength: ai.wavelength,
            pixel_size_1: ai.pixel_size_1,
            pixel_size_2: ai.pixel_size_2,
        }
    }
}

/// Result of azimuthal integration
pub struct IntegrationResult {
    /// Two-theta values (degrees) [n_bins]
    pub two_theta_deg: Array1<f64>,
    
    /// Azimuthal angle values (degrees) [n_chi_bins]
    pub chi_deg: Array1<f64>,
    
    /// Integrated intensity [n_bins, n_chi_bins]
    pub intensity: Array2<f64>,
    
    /// Pixel count per bin [n_bins, n_chi_bins]
    pub counts: Array2<u32>,
}

/// Perform azimuthal integration
/// 
/// Converts detector coordinates (i, j) to reciprocal space (2θ, χ)
/// and integrates intensity into 2D histogram
/// 
/// NOTE: Geometry follows PyFAI conventions:
/// - Detector coordinates: origin at beam center (PONI)
/// - poni1 = vertical offset (in meters)
/// - poni2 = horizontal offset (in meters)
/// - 2θ = 2.0 * atan(r_transverse / distance) [NOT using sin formula]
pub fn azimuthal_integrate(
    data: &ArrayView2<f32>,
    geom: &IntegrationGeometry,
    two_theta_min_deg: f64,
    two_theta_max_deg: f64,
    two_theta_bins: usize,
    chi_min_deg: f64,
    chi_max_deg: f64,
    chi_bins: usize,
) -> ProcessingResult<IntegrationResult> {
    // Convert angles to radians
    let two_theta_min = two_theta_min_deg.to_radians();
    let two_theta_max = two_theta_max_deg.to_radians();
    let chi_min = chi_min_deg.to_radians();
    let chi_max = chi_max_deg.to_radians();
    
    if two_theta_min >= two_theta_max || chi_min > chi_max {
        return Err(ProcessingError::InvalidParameter(
            "Invalid angle ranges".to_string()
        ));
    }
    
    let mut intensity = Array2::zeros((two_theta_bins, chi_bins));
    let mut counts = Array2::from_elem((two_theta_bins, chi_bins), 0u32);
    
    // Pre-compute rotation matrix components
    // For detector rotations: apply Rz(rot3) * Ry(rot2) * Rx(rot1)
    let c1 = geom.rot1.cos();
    let s1 = geom.rot1.sin();
    let c2 = geom.rot2.cos();
    let s2 = geom.rot2.sin();
    let c3 = geom.rot3.cos();
    let s3 = geom.rot3.sin();
    
    // 3x3 rotation matrix for detector coordinate transformation
    // Applied to (x_pixel, y_pixel, 0) -> (x_rot, y_rot, z_rot)
    let r11 = c2 * c3;
    let r12 = s1 * s2 * c3 - c1 * s3;
    let r21 = c2 * s3;
    let r22 = s1 * s2 * s3 + c1 * c3;
    let r31 = -s2;
    let r32 = s1 * c2;
    
    // Iterate over all detector pixels
    let (n_rows, n_cols) = data.dim();
    
    for i in 0..n_rows {
        for j in 0..n_cols {
            let pixel_value = data[[i, j]];
            
            if !pixel_value.is_finite() || pixel_value <= 0.0 {
                continue;
            }
            
            // Convert pixel indices to real detector coordinates (meters)
            // poni1 is the vertical beam center position (meters)
            // poni2 is the horizontal beam center position (meters)
            let y_pixel_m = i as f64 * geom.pixel_size_1 - geom.poni1;
            let x_pixel_m = j as f64 * geom.pixel_size_2 - geom.poni2;
            
            // Apply rotations to detector coordinates
            // Detector is at z = distance, pixel displacement at (x_pixel_m, y_pixel_m, 0)
            // After rotation, we get the position in the rotated frame
            let x_rot = r11 * x_pixel_m + r12 * y_pixel_m;
            let y_rot = r21 * x_pixel_m + r22 * y_pixel_m;
            let z_component = r31 * x_pixel_m + r32 * y_pixel_m;  // z-displacement from detector rotation
            
            // The actual z-position in 3D is: distance + z_component (detector is tilted)
            // But for a flat detector with small rotations, z_component is typically << distance
            let z_pos = geom.distance + z_component;
            
            // Transverse distance from the beam axis (in rotated frame)
            let r_transverse = (x_rot * x_rot + y_rot * y_rot).sqrt();
            
            // Scattering angle 2θ
            // CORRECT FORMULA (matches PyFAI):
            // theta = atan(r_transverse / z_pos)
            // two_theta = 2 * theta
            // This is the arctan formula, NOT sine formula
            let two_theta = 2.0 * (r_transverse / z_pos).atan();
            
            // Azimuthal angle χ
            // atan2(y, x) gives angle from x-axis
            let chi = y_rot.atan2(x_rot);
            
            // Normalize chi to [0, 2π) range
            let chi_normalized = if chi < 0.0 { chi + 2.0 * PI } else { chi };
            
            // Check if within integration range
            if two_theta < two_theta_min || two_theta > two_theta_max {
                continue;
            }
            
            // Check if chi is within range
            if chi_normalized < chi_min || chi_normalized > chi_max {
                continue;
            }
            
            // Map to histogram bins
            let two_theta_bin = ((two_theta - two_theta_min) / (two_theta_max - two_theta_min)
                * two_theta_bins as f64) as usize;
            let chi_bin = ((chi_normalized - chi_min) / (chi_max - chi_min)
                * chi_bins as f64) as usize;
            
            if two_theta_bin < two_theta_bins && chi_bin < chi_bins {
                intensity[[two_theta_bin, chi_bin]] += pixel_value as f64;
                counts[[two_theta_bin, chi_bin]] += 1;
            }
        }
    }
    
    // Generate output angle arrays
    let two_theta_deg = Array1::linspace(two_theta_min_deg, two_theta_max_deg, two_theta_bins);
    let chi_deg = Array1::linspace(chi_min_deg, chi_max_deg, chi_bins);
    
    Ok(IntegrationResult {
        two_theta_deg,
        chi_deg,
        intensity,
        counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr2;
    
    #[test]
    fn test_apply_flatfield() {
        let image = arr2(&[[100u32, 200], [300, 400]]);
        let flatfield = arr2(&[[1.0f32, 2.0], [1.0, 2.0]]);
        
        let corrected = apply_flatfield(&image.view(), &flatfield.view()).unwrap();
        
        assert_eq!(corrected[[0, 0]], 100.0);
        assert_eq!(corrected[[0, 1]], 100.0);
        assert_eq!(corrected[[1, 0]], 300.0);
        assert_eq!(corrected[[1, 1]], 200.0);
    }
    
    #[test]
    fn test_fractile_filter() {
        let data = arr2(&[
            [1.0f32, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        
        let (filtered, low_mask, high_mask) = 
            fractile_filter(&data.view(), 25.0, 75.0).unwrap();
        
        // Check that some values were clipped
        assert!(low_mask.iter().any(|&x| x));
        assert!(high_mask.iter().any(|&x| x));
        
        // Check that filtered values are bounded
        let max_val = filtered.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        assert!(max_val >= 1.0 && max_val <= 16.0);
    }
    
    #[test]
    fn test_integration_geometry() {
        let ai = AzimuthalIntegrator {
            detector: "Test".to_string(),
            detector_config: Default::default(),
            distance: 1.55,
            poni1: 0.318,
            poni2: 0.278,
            rot1: 0.0036,
            rot2: -0.00096,
            rot3: 0.0,
            wavelength: 1.65e-11,
            pixel_size_1: 150e-6,
            pixel_size_2: 150e-6,
            poni_version: "2.1".to_string(),
        };
        
        let geom = IntegrationGeometry::from(&ai);
        assert_eq!(geom.distance, 1.55);
        assert_eq!(geom.wavelength, 1.65e-11);
    }
}
