use pilatus4_explorer::*;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = "/Users/openclaw/.openclaw/workspace/test_data/pilatus4";
    
    println!("=== PILATUS4 Data Explorer ===\n");
    
    // Load and display PONI calibration
    println!("Loading calibration from PONI file...");
    let ai = parse_poni(Path::new(&format!("{}/calibration.poni", test_dir)))?;
    println!("✓ Detector: {}", ai.detector);
    println!("✓ Distance: {:.6} m", ai.distance);
    println!("✓ Wavelength: {:.6e} m", ai.wavelength);
    println!();
    
    // Load bright field correction
    println!("Loading bright field correction...");
    let ff = load_bright_field(Path::new(&format!("{}/bright_field.npy", test_dir)))?;
    println!("✓ Shape: {:?}", ff.dim());
    println!("✓ Range: [{:.3}, {:.3}]", 
             ff.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
             ff.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)));
    println!();
    
    // Load mask
    println!("Loading pixel mask...");
    let mask = load_mask(Path::new(&format!("{}/mask.edf", test_dir)))?;
    println!("✓ Shape: {:?}", mask.dim());
    let masked = mask.iter().filter(|&&x| x).count();
    println!("✓ Masked pixels: {} / {} ({:.2}%)", 
             masked, 2180 * 2073, 
             100.0 * masked as f64 / (2180 * 2073) as f64);
    println!();
    
    // Load detector config (default for now)
    println!("Detector configuration:");
    let config = DetectorConfig::default();
    println!("✓ Name: {}", config.name);
    println!("✓ Dimensions: {} × {}", config.n_rows, config.n_cols);
    println!("✓ Pixel size: {:.0} µm × {:.0} µm", 
             config.pixel_size_1 * 1e6, config.pixel_size_2 * 1e6);
    println!();
    
    // Load sample image
    println!("Loading sample image...");
    let image = load_tiff(Path::new(&format!("{}/sample_lab6.tiff", test_dir)))?;
    println!("✓ Shape: {:?}", image.dim());
    let min = image.iter().fold(u32::MAX, |a, &b| a.min(b));
    let max = image.iter().fold(0u32, |a, &b| a.max(b));
    let mean = image.iter().map(|&x| x as f64).sum::<f64>() / (2180 * 2073) as f64;
    println!("✓ Range: [{}, {}], Mean: {:.1}", min, max, mean);
    println!();
    
    println!("=== All data loaded successfully ===");
    
    Ok(())
}
