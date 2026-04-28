//! CLI tool for PILATUS4 data processing and integration
//! 
//! Integrates Phase 1-3 functionality:
//! - Phase 1: Load geometry from PONI file
//! - Phase 2: Load detector mask and flatfield
//! - Phase 3: Apply corrections and integrate

use pilatus4_explorer::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Serialize, Deserialize)]
struct CliResult {
    /// Input file
    image_path: String,
    
    /// Image shape
    image_shape: (usize, usize),
    
    /// Image statistics
    image_stats: ImageStats,
    
    /// Detector mask statistics
    mask_stats: MaskStats,
    
    /// Flatfield statistics
    flatfield_stats: FlatfieldStats,
    
    /// Processing results
    processing: ProcessingStats,
    
    /// Integration results
    integration: IntegrationStats,
    
    /// Performance metrics
    performance: PerformanceMetrics,
}

#[derive(Serialize, Deserialize)]
struct ImageStats {
    min: u32,
    max: u32,
    mean: f64,
    total_counts: u64,
}

#[derive(Serialize, Deserialize)]
struct MaskStats {
    total_pixels: usize,
    masked_pixels: usize,
    masked_fraction: f64,
}

#[derive(Serialize, Deserialize)]
struct FlatfieldStats {
    min: f32,
    max: f32,
    mean: f32,
}

#[derive(Serialize, Deserialize)]
struct ProcessingStats {
    flatfield_applied: bool,
    low_percentile: f32,
    high_percentile: f32,
    low_clipped: usize,
    high_clipped: usize,
}

#[derive(Serialize, Deserialize)]
struct IntegrationStats {
    two_theta_min_deg: f64,
    two_theta_max_deg: f64,
    two_theta_bins: usize,
    chi_min_deg: f64,
    chi_max_deg: f64,
    chi_bins: usize,
    total_intensity: f64,
    total_counts: u64,
}

#[derive(Serialize, Deserialize)]
struct PerformanceMetrics {
    io_loading_ms: u128,
    flatfield_correction_ms: u128,
    fractile_filtering_ms: u128,
    integration_ms: u128,
    total_ms: u128,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_total = Instant::now();
    
    // Configuration
    let test_data_dir = "/Users/openclaw/.openclaw/workspace/test_data/pilatus4";
    let output_dir = "/Users/openclaw/.openclaw/workspace";
    
    println!("=== PILATUS4 Explorer CLI - Phase 4 Integration ===\n");
    
    // =====================================================
    // Phase 1: Load geometry from PONI
    // =====================================================
    println!("[Phase 1/4] Loading geometry from PONI...");
    let start_io = Instant::now();
    
    let poni_path = format!("{}/calibration.poni", test_data_dir);
    let ai = parse_poni(Path::new(&poni_path))?;
    println!("  ✓ Detector: {}", ai.detector);
    println!("  ✓ Distance: {:.6} m", ai.distance);
    println!("  ✓ Wavelength: {:.6e} m", ai.wavelength);
    
    // =====================================================
    // Phase 2: Load detector mask and flatfield
    // =====================================================
    println!("\n[Phase 2/4] Loading detector mask and flatfield...");
    
    let mask_path = format!("{}/mask.edf", test_data_dir);
    let mask = load_mask(Path::new(&mask_path))?;
    let masked_pixels: usize = mask.iter().filter(|&&x| x).count();
    let total_pixels = 2180 * 2073;
    
    println!("  ✓ Mask shape: {:?}", mask.dim());
    println!("  ✓ Masked pixels: {}/{} ({:.2}%)",
             masked_pixels, total_pixels,
             100.0 * masked_pixels as f64 / total_pixels as f64);
    
    let ff_path = format!("{}/bright_field.npy", test_data_dir);
    let flatfield = load_bright_field(Path::new(&ff_path))?;
    let ff_min = flatfield.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let ff_max = flatfield.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let ff_mean = flatfield.iter().sum::<f32>() / flatfield.len() as f32;
    
    println!("  ✓ Flatfield shape: {:?}", flatfield.dim());
    println!("  ✓ Flatfield range: [{:.3}, {:.3}]", ff_min, ff_max);
    println!("  ✓ Flatfield mean: {:.3}", ff_mean);
    
    let io_ms = start_io.elapsed().as_millis();
    
    // =====================================================
    // Phase 3: Apply corrections and integrate
    // =====================================================
    println!("\n[Phase 3/4] Processing data...");
    
    // Process both test images
    let images = vec!["sample_lab6.tiff", "sample_ceo2.tiff"];
    
    for (image_idx, image_name) in images.iter().enumerate() {
        let image_path = format!("{}/{}", test_data_dir, image_name);
        
        println!("\n  Processing {}...", image_name);
        let start_proc = Instant::now();
        
        // Load image
        let image = load_tiff(Path::new(&image_path))?;
        let img_view = image.view();
        
        // Image statistics
        let img_min = image.iter().fold(u32::MAX, |a, &b| a.min(b));
        let img_max = image.iter().fold(0u32, |a, &b| a.max(b));
        let img_sum: u64 = image.iter().map(|&x| x as u64).sum();
        let img_mean = img_sum as f64 / image.len() as f64;
        
        println!("    Shape: {:?}, Range: [{}, {}], Mean: {:.1}",
                 image.dim(), img_min, img_max, img_mean);
        
        // Apply flatfield correction
        let start_ff = Instant::now();
        let corrected = apply_flatfield(&img_view, &flatfield.view())?;
        let ff_ms = start_ff.elapsed().as_millis();
        
        println!("    ✓ Flatfield correction applied ({} ms)", ff_ms);
        
        // Apply fractile filtering
        let start_frac = Instant::now();
        let (filtered, low_mask, high_mask) = 
            fractile_filter(&corrected.view(), 5.0, 95.0)?;
        let frac_ms = start_frac.elapsed().as_millis();
        
        let low_count = low_mask.iter().filter(|&&x| x).count();
        let high_count = high_mask.iter().filter(|&&x| x).count();
        
        println!("    ✓ Fractile filtering applied ({} ms)", frac_ms);
        println!("      Low clipped: {}, High clipped: {}", low_count, high_count);
        
        // Azimuthal integration
        let start_int = Instant::now();
        let geom = IntegrationGeometry::from(&ai);
        
        let result = azimuthal_integrate(
            &filtered.view(),
            &geom,
            2.0,   // two_theta_min
            25.0,  // two_theta_max
            100,   // two_theta_bins
            -180.0, // chi_min
            180.0,  // chi_max
            100,   // chi_bins
        )?;
        
        let int_ms = start_int.elapsed().as_millis();
        
        let total_intensity: f64 = result.intensity.iter().sum();
        let total_counts: u64 = result.counts.iter().map(|&x| x as u64).sum();
        
        println!("    ✓ Azimuthal integration completed ({} ms)", int_ms);
        println!("      Total intensity: {:.2e}", total_intensity);
        println!("      Total counts: {}", total_counts);
        
        // =====================================================
        // Phase 4: Export results to JSON
        // =====================================================
        println!("\n[Phase 4/4] Exporting results to JSON...");
        
        let total_proc_ms = start_proc.elapsed().as_millis();
        
        let cli_result = CliResult {
            image_path: image_path.clone(),
            image_shape: image.dim(),
            image_stats: ImageStats {
                min: img_min,
                max: img_max,
                mean: img_mean,
                total_counts: img_sum,
            },
            mask_stats: MaskStats {
                total_pixels,
                masked_pixels,
                masked_fraction: masked_pixels as f64 / total_pixels as f64,
            },
            flatfield_stats: FlatfieldStats {
                min: ff_min,
                max: ff_max,
                mean: ff_mean,
            },
            processing: ProcessingStats {
                flatfield_applied: true,
                low_percentile: 5.0,
                high_percentile: 95.0,
                low_clipped: low_count,
                high_clipped: high_count,
            },
            integration: IntegrationStats {
                two_theta_min_deg: 2.0,
                two_theta_max_deg: 25.0,
                two_theta_bins: 100,
                chi_min_deg: -180.0,
                chi_max_deg: 180.0,
                chi_bins: 100,
                total_intensity,
                total_counts,
            },
            performance: PerformanceMetrics {
                io_loading_ms: io_ms,
                flatfield_correction_ms: ff_ms,
                fractile_filtering_ms: frac_ms,
                integration_ms: int_ms,
                total_ms: total_proc_ms,
            },
        };
        
        // Write to JSON file
        let output_filename = format!("test_results_{}.json", 
                                     image_name.replace(".tiff", ""));
        let output_path = format!("{}/{}", output_dir, output_filename);
        
        let json_str = serde_json::to_string_pretty(&cli_result)?;
        let mut file = File::create(&output_path)?;
        file.write_all(json_str.as_bytes())?;
        
        println!("  ✓ Results exported to {}", output_path);
        
        // Print summary
        println!("\n  === Test Summary ===");
        println!("    Total processing time: {} ms", total_proc_ms);
        println!("    Image: {} ({}x{})", image_name, image.dim().0, image.dim().1);
        println!("    Masked pixels: {} ({:.2}%)", masked_pixels, 
                 100.0 * masked_pixels as f64 / total_pixels as f64);
        println!("    Integration bins: {} × {}", 100, 100);
        println!("    Output: {}", output_path);
    }
    
    let total_ms = start_total.elapsed().as_millis();
    
    println!("\n=== Phase 4 Integration Testing Complete ===");
    println!("Total execution time: {} ms", total_ms);
    println!("Status: ✓ All phases completed successfully");
    
    Ok(())
}
