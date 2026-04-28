//! PILATUS4 data explorer library
//! 
//! This library provides file I/O and processing for PILATUS4 detector data.
//! Phase 1-2: File I/O loaders for NumPy, PONI, EDF, and TIFF formats.
//! Phase 3: Data processing (flatfield correction, filtering, integration).
//! Phase 5: WASM bindings for browser-based exploration.

pub mod io;
pub mod processing;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use io::{
    load_bright_field, load_detector_config, parse_poni, load_mask, load_tiff,
    AzimuthalIntegrator, DetectorConfig, LoadError, LoadResult,
};

pub use processing::{
    apply_flatfield, fractile_filter, azimuthal_integrate,
    IntegrationGeometry, IntegrationResult, ProcessingError, ProcessingResult,
};

#[cfg(target_arch = "wasm32")]
pub use wasm::DataExplorer;
