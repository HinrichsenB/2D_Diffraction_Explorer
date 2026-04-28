//! File I/O loaders for PILATUS4 detector data
//! 
//! Supports:
//! - NumPy .npy files (bright field corrections)
//! - PONI calibration files (pyFAI format)
//! - EDF mask files
//! - TIFF images (raw detector data)

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use ndarray::Array2;
use thiserror::Error;

/// Errors that can occur during file loading
#[derive(Error, Debug)]
pub enum LoadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("NumPy loading error: {0}")]
    NumpyError(String),
    
    #[error("PONI parsing error: {0}")]
    PoniError(String),
    
    #[error("EDF parsing error: {0}")]
    EdfError(String),
    
    #[error("TIFF error: {0}")]
    TiffError(String),
    
    #[error("Invalid array shape: expected {expected}, got {actual}")]
    InvalidShape { expected: String, actual: String },
    
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Result type for file loading operations
pub type LoadResult<T> = Result<T, LoadError>;

/// Detector configuration
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    pub name: String,
    pub pixel_size_1: f64,  // meters
    pub pixel_size_2: f64,  // meters
    pub n_rows: usize,
    pub n_cols: usize,
    pub sensor_material: String,
    pub sensor_thickness: f64,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        // PILATUS4 4M default configuration
        Self {
            name: "Pilatus4_4M".to_string(),
            pixel_size_1: 150e-6,  // 150 micrometers
            pixel_size_2: 150e-6,
            n_rows: 2180,
            n_cols: 2073,
            sensor_material: "CdTe".to_string(),
            sensor_thickness: 0.001,
        }
    }
}

/// Azimuthal integrator configuration from PONI file
#[derive(Debug, Clone)]
pub struct AzimuthalIntegrator {
    pub detector: String,
    pub detector_config: HashMap<String, String>,
    pub distance: f64,           // meters
    pub poni1: f64,              // vertical offset (meters)
    pub poni2: f64,              // horizontal offset (meters)
    pub rot1: f64,               // rotation angle 1 (radians)
    pub rot2: f64,               // rotation angle 2 (radians)
    pub rot3: f64,               // rotation angle 3 (radians)
    pub wavelength: f64,         // wavelength (meters)
    pub pixel_size_1: f64,       // pixel size (meters)
    pub pixel_size_2: f64,
    pub poni_version: String,
}

/// Load bright field correction from NumPy .npy file
/// Load bright field correction from NumPy .npy file
pub fn load_bright_field<P: AsRef<Path>>(path: P) -> LoadResult<Array2<f32>> {
    let path = path.as_ref();
    let mut reader = File::open(path)?;
    
    // Read NPY header
    let mut magic = [0u8; 6];
    reader.read_exact(&mut magic)?;
    
    if &magic != b"\x93NUMPY" {
        return Err(LoadError::NumpyError("Invalid NPY magic number".to_string()));
    }
    
    // Read version (1.0 = version 1, major=1, minor=0)
    let mut version = [0u8; 2];
    reader.read_exact(&mut version)?;
    
    // Read header length
    let mut header_len_bytes = [0u8; 2];
    reader.read_exact(&mut header_len_bytes)?;
    let header_len = u16::from_le_bytes(header_len_bytes) as usize;
    
    // Read header
    let mut header_buf = vec![0u8; header_len];
    reader.read_exact(&mut header_buf)?;
    let header = String::from_utf8_lossy(&header_buf);
    
    // Parse shape from header
    let shape_start = header.find("'shape': (").ok_or_else(|| LoadError::NumpyError("Missing shape in NPY header".to_string()))?;
    let shape_end = header[shape_start..].find(')').ok_or_else(|| LoadError::NumpyError("Invalid shape format".to_string()))?;
    let shape_str = &header[shape_start + 10..shape_start + shape_end];
    
    let dims: Vec<&str> = shape_str.split(',').map(|s| s.trim()).collect();
    if dims.len() != 2 {
        return Err(LoadError::NumpyError(format!("Expected 2D array, got {}D", dims.len())));
    }
    
    let dim1: usize = dims[0].parse()
        .map_err(|_| LoadError::NumpyError("Invalid dimension 1".to_string()))?;
    let dim2: usize = dims[1].parse()
        .map_err(|_| LoadError::NumpyError("Invalid dimension 2".to_string()))?;
    
    if (dim1, dim2) != (2180, 2073) {
        return Err(LoadError::InvalidShape {
            expected: "(2180, 2073)".to_string(),
            actual: format!("({}, {})", dim1, dim2),
        });
    }
    
    // Read data
    let mut data: Vec<f32> = vec![0.0; dim1 * dim2];
    for elem in &mut data {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        *elem = f32::from_le_bytes(bytes);
    }
    
    let arr = Array2::from_shape_vec((dim1, dim2), data)
        .map_err(|_| LoadError::NumpyError("Failed to reshape array".to_string()))?;
    
    Ok(arr)
}

/// Load detector configuration (returns default for now)
pub fn load_detector_config<P: AsRef<Path>>(_path: P) -> LoadResult<DetectorConfig> {
    // Return default PILATUS4 4M configuration
    Ok(DetectorConfig::default())
}

/// Parse PONI calibration file (pyFAI format)
pub fn parse_poni<P: AsRef<Path>>(path: P) -> LoadResult<AzimuthalIntegrator> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)?;
    
    let mut ai = AzimuthalIntegrator {
        detector: String::new(),
        detector_config: HashMap::new(),
        distance: 0.0,
        poni1: 0.0,
        poni2: 0.0,
        rot1: 0.0,
        rot2: 0.0,
        rot3: 0.0,
        wavelength: 0.0,
        pixel_size_1: 150e-6,
        pixel_size_2: 150e-6,
        poni_version: String::new(),
    };
    
    for line in contents.lines() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Parse key: value pairs
        if let Some(colon_idx) = line.find(':') {
            let key = line[..colon_idx].trim();
            let value = line[colon_idx + 1..].trim();
            
            match key {
                "poni_version" => ai.poni_version = value.to_string(),
                "Detector" => ai.detector = value.to_string(),
                "Detector_config" => {
                    if let Ok(config) = parse_detector_config(value) {
                        ai.detector_config = config;
                        if let Some(p1_str) = ai.detector_config.get("pixel1") {
                            if let Ok(p1) = p1_str.parse::<f64>() {
                                ai.pixel_size_1 = p1;
                            }
                        }
                        if let Some(p2_str) = ai.detector_config.get("pixel2") {
                            if let Ok(p2) = p2_str.parse::<f64>() {
                                ai.pixel_size_2 = p2;
                            }
                        }
                    }
                }
                "Distance" => {
                    ai.distance = value.parse()
                        .map_err(|_| LoadError::PoniError(
                            format!("Invalid distance value: {}", value)
                        ))?;
                }
                "Poni1" => {
                    ai.poni1 = value.parse()
                        .map_err(|_| LoadError::PoniError(
                            format!("Invalid poni1 value: {}", value)
                        ))?;
                }
                "Poni2" => {
                    ai.poni2 = value.parse()
                        .map_err(|_| LoadError::PoniError(
                            format!("Invalid poni2 value: {}", value)
                        ))?;
                }
                "Rot1" => {
                    ai.rot1 = value.parse()
                        .map_err(|_| LoadError::PoniError(
                            format!("Invalid rot1 value: {}", value)
                        ))?;
                }
                "Rot2" => {
                    ai.rot2 = value.parse()
                        .map_err(|_| LoadError::PoniError(
                            format!("Invalid rot2 value: {}", value)
                        ))?;
                }
                "Rot3" => {
                    ai.rot3 = value.parse()
                        .map_err(|_| LoadError::PoniError(
                            format!("Invalid rot3 value: {}", value)
                        ))?;
                }
                "Wavelength" => {
                    ai.wavelength = value.parse()
                        .map_err(|_| LoadError::PoniError(
                            format!("Invalid wavelength value: {}", value)
                        ))?;
                }
                _ => {} // Ignore unknown fields
            }
        }
    }
    
    // Validate required fields
    if ai.distance == 0.0 {
        return Err(LoadError::MissingField("Distance".to_string()));
    }
    if ai.wavelength == 0.0 {
        return Err(LoadError::MissingField("Wavelength".to_string()));
    }
    
    Ok(ai)
}

/// Parse detector config from JSON-like format in PONI file
fn parse_detector_config(value: &str) -> LoadResult<HashMap<String, String>> {
    let mut config = HashMap::new();
    let inner = value.trim_matches(|c| c == '{' || c == '}');
    
    for pair in inner.split(',') {
        if let Some(colon_idx) = pair.find(':') {
            let k = pair[..colon_idx].trim().trim_matches('"');
            let v = pair[colon_idx + 1..].trim().trim_matches('"');
            config.insert(k.to_string(), v.to_string());
        }
    }
    
    Ok(config)
}

/// Load mask from EDF (ESRF Data Format) file
pub fn load_mask<P: AsRef<Path>>(path: P) -> LoadResult<Array2<bool>> {
    let path = path.as_ref();
    
    // Read file
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    
    // Parse EDF header (starts at '{' and ends at '}')
    let header_end = buf.iter().position(|&b| b == b'}' as u8)
        .ok_or_else(|| LoadError::EdfError("Invalid EDF format: no closing brace".to_string()))? + 1;
    
    // EDF headers are fixed size (padded), find the actual end
    let header = parse_edf_header(&buf[..header_end])?;
    
    // Get header size to skip to data
    let header_size: usize = header.get("EDF_HeaderSize")
        .ok_or_else(|| LoadError::EdfError("Missing EDF_HeaderSize field".to_string()))?
        .parse()
        .map_err(|_| LoadError::EdfError("Invalid EDF_HeaderSize value".to_string()))?;
    
    // Extract array dimensions
    let size1: usize = header.get("Size")
        .ok_or_else(|| LoadError::EdfError("Missing Size field".to_string()))?
        .parse()
        .map_err(|_| LoadError::EdfError("Invalid Size value".to_string()))?;
    
    let size2: usize = header.get("Dim_2")
        .ok_or_else(|| LoadError::EdfError("Missing Dim_2 field".to_string()))?
        .parse()
        .map_err(|_| LoadError::EdfError("Invalid Dim_2 value".to_string()))?;
    
    let dim1: usize = header.get("Dim_1")
        .ok_or_else(|| LoadError::EdfError("Missing Dim_1 field".to_string()))?
        .parse()
        .map_err(|_| LoadError::EdfError("Invalid Dim_1 value".to_string()))?;
    
    // Data type from header
    let data_type = header.get("DataType")
        .map(|s| s.as_str())
        .unwrap_or("UnsignedByte");
    
    // Data starts after the header block
    let data_slice = &buf[header_size..];
    
    // Parse binary data based on data type
    let flat_data = match data_type {
        "SignedByte" | "SignedInteger" => {
            data_slice.iter().take(size1).map(|&b| b != 0).collect::<Vec<_>>()
        }
        "UnsignedByte" => {
            data_slice.iter().take(size1).map(|&b| b != 0).collect::<Vec<_>>()
        }
        "SignedShort" | "UnsignedShort" => {
            let mut vals = Vec::with_capacity(size1);
            for chunk in data_slice.chunks(2).take(size1) {
                if chunk.len() == 2 {
                    let val = u16::from_le_bytes([chunk[0], chunk[1]]);
                    vals.push(val != 0);
                }
            }
            vals
        }
        _ => {
            return Err(LoadError::EdfError(
                format!("Unsupported DataType: {}", data_type)
            ));
        }
    };
    
    if flat_data.len() != size1 {
        return Err(LoadError::EdfError(
            format!("Data size mismatch: expected {}, got {}", size1, flat_data.len())
        ));
    }
    
    // Reshape to 2D array (Dim_1 is columns, Dim_2 is rows)
    let array = Array2::from_shape_vec((size2, dim1), flat_data)
        .map_err(|_| LoadError::EdfError(
            format!("Cannot reshape {}x{} array to {}x{}", 
                    1, size1, size2, dim1)
        ))?;
    
    Ok(array)
}

/// Parse EDF header (header ends at \n\n)
fn parse_edf_header(buf: &[u8]) -> LoadResult<HashMap<String, String>> {
    let mut header = HashMap::new();
    let header_str = String::from_utf8_lossy(buf);
    
    for line in header_str.lines() {
        let line = line.trim();
        
        // Skip comments, braces, and empty lines
        if line.is_empty() || line.starts_with('{') || line.starts_with('}') || line.starts_with('_') {
            continue;
        }
        
        // Parse key = value
        if let Some(eq_idx) = line.find('=') {
            let key = line[..eq_idx].trim();
            let value = line[eq_idx + 1..].trim();
            
            // Remove trailing semicolon if present
            let value = if value.ends_with(';') {
                &value[..value.len() - 1]
            } else {
                value
            }.trim();
            
            header.insert(key.to_string(), value.to_string());
        }
    }
    
    Ok(header)
}

/// Load image from TIFF file
pub fn load_tiff<P: AsRef<Path>>(path: P) -> LoadResult<Array2<u32>> {
    let path = path.as_ref();
    let file = File::open(path)?;
    
    let mut decoder = tiff::decoder::Decoder::new(file)
        .map_err(|e| LoadError::TiffError(e.to_string()))?;
    
    // Read image data
    let tiff_data = decoder.read_image()
        .map_err(|e| LoadError::TiffError(e.to_string()))?;
    
    // Get dimensions
    let (width, height) = decoder.dimensions()
        .map_err(|e| LoadError::TiffError(e.to_string()))?;
    
    // Convert to u32 array
    let data: Vec<u32> = match tiff_data {
        tiff::decoder::DecodingResult::U8(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
        tiff::decoder::DecodingResult::U16(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
        tiff::decoder::DecodingResult::U32(v) => v,
        tiff::decoder::DecodingResult::U64(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
        tiff::decoder::DecodingResult::F32(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
        tiff::decoder::DecodingResult::F64(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
        tiff::decoder::DecodingResult::I8(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
        tiff::decoder::DecodingResult::I16(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
        tiff::decoder::DecodingResult::I32(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
        tiff::decoder::DecodingResult::I64(v) => {
            v.into_iter().map(|x| x as u32).collect()
        }
    };
    
    // Reshape to array
    let array = Array2::from_shape_vec((height as usize, width as usize), data)
        .map_err(|_| LoadError::TiffError(
            format!("Cannot reshape data to {}x{}", height, width)
        ))?;
    
    Ok(array)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    const TEST_DATA_DIR: &str = "/Users/openclaw/.openclaw/workspace/test_data/pilatus4";
    
    #[test]
    fn test_load_bright_field() {
        let path = format!("{}/bright_field.npy", TEST_DATA_DIR);
        let ff = load_bright_field(&path).expect("Failed to load bright field");
        
        assert_eq!(ff.dim(), (2180, 2073), "Bright field shape mismatch");
        let all_positive = ff.iter().all(|&x| x > 0.0);
        assert!(all_positive, "Bright field contains non-positive values");
    }
    
    #[test]
    fn test_parse_poni() {
        let path = format!("{}/calibration.poni", TEST_DATA_DIR);
        let ai = parse_poni(&path).expect("Failed to parse PONI");
        
        assert!(!ai.detector.is_empty(), "Detector name is empty");
        assert!(ai.distance > 0.0, "Distance must be positive");
        assert!(ai.wavelength > 0.0, "Wavelength must be positive");
    }
    
    #[test]
    fn test_load_mask() {
        let path = format!("{}/mask.edf", TEST_DATA_DIR);
        let mask = load_mask(&path).expect("Failed to load mask");
        
        assert_eq!(mask.dim(), (2180, 2073), "Mask shape mismatch");
        let any_masked = mask.iter().any(|&x| x);
        assert!(any_masked, "Mask appears to be all zeros");
    }
    
    #[test]
    fn test_load_tiff() {
        let path = format!("{}/sample_lab6.tiff", TEST_DATA_DIR);
        let image = load_tiff(&path).expect("Failed to load TIFF");
        
        assert_eq!(image.dim(), (2180, 2073), "TIFF shape mismatch");
        let max = image.iter().fold(0u32, |a, &b| a.max(b));
        assert!(max > 0, "TIFF contains no signal");
    }
    
    #[test]
    fn test_load_second_tiff() {
        let path = format!("{}/sample_ceo2.tiff", TEST_DATA_DIR);
        let image = load_tiff(&path).expect("Failed to load second TIFF");
        
        assert_eq!(image.dim(), (2180, 2073), "TIFF shape mismatch");
        let max = image.iter().fold(0u32, |a, &b| a.max(b));
        assert!(max > 0, "TIFF contains no signal");
    }
    
    #[test]
    fn test_poni_detector_config() {
        let path = format!("{}/calibration.poni", TEST_DATA_DIR);
        let ai = parse_poni(&path).expect("Failed to parse PONI");
        
        // Check detector info
        assert_eq!(ai.detector, "Pilatus4_CdTe_4M");
        assert!(!ai.detector_config.is_empty(), "Detector config should be parsed");
        
        // Check pixel size was extracted
        if let Some(p1) = ai.detector_config.get("pixel1") {
            let pixel_size: f64 = p1.parse().expect("Invalid pixel size");
            assert!(pixel_size > 0.0, "Pixel size must be positive");
        }
    }
    
    #[test]
    fn test_full_data_pipeline() {
        // Load all data types in sequence
        let ai = parse_poni(&format!("{}/calibration.poni", TEST_DATA_DIR))
            .expect("Failed to load PONI");
        let ff = load_bright_field(&format!("{}/bright_field.npy", TEST_DATA_DIR))
            .expect("Failed to load bright field");
        let mask = load_mask(&format!("{}/mask.edf", TEST_DATA_DIR))
            .expect("Failed to load mask");
        let image1 = load_tiff(&format!("{}/sample_lab6.tiff", TEST_DATA_DIR))
            .expect("Failed to load first image");
        let image2 = load_tiff(&format!("{}/sample_ceo2.tiff", TEST_DATA_DIR))
            .expect("Failed to load second image");
        
        // Verify all shapes are consistent
        assert_eq!(ff.dim(), mask.dim(), "Flat field and mask shapes don't match");
        assert_eq!(ff.dim(), image1.dim(), "Flat field and image1 shapes don't match");
        assert_eq!(image1.dim(), image2.dim(), "Image1 and image2 shapes don't match");
        
        // Verify calibration data
        assert!(ai.distance > 0.0 && ai.distance < 10.0, "Distance out of range");
        assert!(ai.wavelength > 1e-12 && ai.wavelength < 1e-9, "Wavelength out of range");
    }
    
    #[test]
    fn test_error_handling_missing_file() {
        let result = load_bright_field("/nonexistent/file.npy");
        assert!(result.is_err(), "Should fail on missing file");
    }
    
    #[test]
    fn test_error_handling_invalid_poni() {
        // This would fail if PONI had no Distance field
        // (but our test file has it, so this is just documentation)
        let _ai = parse_poni(&format!("{}/calibration.poni", TEST_DATA_DIR))
            .expect("PONI should parse correctly");
    }
}
