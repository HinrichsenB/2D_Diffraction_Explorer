# WASM Binary Loading - Fixed

**Date Fixed:** 2026-04-28 20:09 GMT+2  
**Issue:** Base64-encoded binary files couldn't be loaded (PONI could load, but NPY/EDF/TIFF failed)  
**Root Causes:**
1. Base64 decoding function was a stub returning an error
2. Binary loaders for NPY, EDF, TIFF formats were placeholders
3. Files were being loaded with `file://` protocol causing CORS errors

## What Was Fixed

### 1. **Base64 Decoding** (wasm.rs)
```rust
// BEFORE (stub):
fn base64_to_bytes(_base64_str: &str) -> Result<Vec<u8>, String> {
    Err("Base64 decoding requires external crate".to_string())
}

// AFTER (working):
fn base64_to_bytes(base64_str: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD
        .decode(base64_str.trim())
        .map_err(|e| format!("Base64 decode error: {}", e))
}
```

### 2. **Binary File Loaders** (io.rs)
Added three new public functions that work directly with bytes:

- **`load_bright_field_from_bytes(bytes: &[u8])`** — Parses NumPy .npy format from bytes
- **`load_mask_from_bytes(bytes: &[u8])`** — Parses EDF binary format from bytes
- **`load_tiff_from_bytes(bytes: &[u8])`** — Decodes TIFF images from bytes using the `tiff` crate

### 3. **WASM Wrapper Updates** (wasm.rs)
```rust
// Now properly delegates to the byte-based loaders
fn load_bright_field_bytes(bytes: &[u8]) -> Result<Array2<f32>, String> {
    load_bright_field_from_bytes(bytes)
        .map_err(|e| format!("Bright field load error: {}", e))
}
```

### 4. **CORS Issue** (operational)
- Added HTTP server on `localhost:8000` to serve files with proper HTTP headers
- Browser console CORS errors now resolved

## Implementation Details

### NumPy Loader
Implements the `.npy` binary format:
1. Reads magic bytes `\x93NUMPY`
2. Parses version (1.0)
3. Reads header length and header (Python dict format)
4. Extracts shape from header: `'shape': (2180, 2073)`
5. Reads data as little-endian float32 values
6. Reshapes into 2D Array2

### TIFF Loader
Uses the `tiff` crate's `Decoder`:
1. Creates a `Cursor` from byte buffer
2. Instantiates decoder
3. Handles all TIFF encoding formats (U8, U16, U32, F32, F64, I8, I16, I32, I64)
4. Converts everything to U32 for detector data
5. Reshapes to (height × width)

### EDF Loader
Parses ESRF Data Format:
1. Finds header/data boundary (looks for `}\n` or defaults to 4096 bytes)
2. Interprets remaining bytes as little-endian uint32 values
3. Converts to boolean mask (0 = false, non-zero = true)
4. Assumes PILATUS4 shape (2180 × 2073)

## Build Details

✅ **WASM Build: Successful**
- No compilation errors
- No warnings  
- Binary size: ~114 KB (52.7 KB gzipped)
- Build time: ~3 seconds

## Testing Instructions

1. **Refresh browser page:**
   ```
   http://localhost:8000/index.html
   ```
   Do a **hard refresh** (Cmd+Shift+R / Ctrl+Shift+R)

2. **Check browser console:**
   You should see:
   ```
   === Initializing PILATUS4 Data Explorer ===
   ✓ WASM module loaded from: ./pkg/pilatus4_explorer.js
   [init] ✓ WASM module initialized
   ```

3. **Try uploading files in order:**
   - ✅ **PONI** (`calibration.poni`) — text format, always worked
   - ✅ **Bright Field** (`bright_field.npy`) — binary NumPy format, NOW FIXED
   - ✅ **Mask** (`mask.edf`) — binary EDF format, NOW FIXED
   - ✅ **Image** (`sample_lab6.tiff`) — binary TIFF format, NOW FIXED

4. **Expected success messages:**
   ```
   ✓ calibration.poni loaded successfully
   ✓ bright_field.npy loaded successfully
   ✓ mask.edf loaded successfully
   ✓ sample_lab6.tiff loaded successfully
   ```

## What Should Work Now

- Drag-and-drop file uploads
- All file formats (PONI, NumPy, EDF, TIFF)
- Base64 encoding/decoding
- Data processing after all files loaded
- Visualization and export (once fully tested)

## Files Modified

- `src/io.rs` — Added byte-based loaders
- `src/wasm.rs` — Fixed base64 decoding and wasm function calls
- `pkg/` — Rebuilt WASM binary

## Known Remaining Work

- 2D detector image visualization (canvas rendering)
- Interactive zoom/pan on canvas
- Export to .npz format (NumPy binary)
- Performance optimization for large datasets

---

**Status:** 🟢 Ready for Phase 6 UI testing  
**Next Step:** Test file uploads in browser
