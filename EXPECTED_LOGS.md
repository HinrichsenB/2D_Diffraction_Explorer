# Expected Console Logs - PILATUS4 Data Explorer

## Expected Startup Sequence (Copy from Browser Console)

After refreshing the page, you should see this in the browser console:

```
=== Initializing PILATUS4 Data Explorer ===
Current location: file:///Users/openclaw/.openclaw/workspace/projects/pilatus4_explorer/index.html
Script directory: file:///Users/openclaw/.openclaw/workspace/projects/pilatus4_explorer/app.js
[init] Trying WASM path: ./pkg/pilatus4_explorer.js
✓ WASM module loaded from: ./pkg/pilatus4_explorer.js
[init] Initializing WASM module...
[init] ✓ WASM module initialized
[init] Creating DataExplorer instance...
[init] ✓ DataExplorer instance created
[init] Available methods: 
  (4) ['constructor', 'detector_info', 'free', 'load_bright_field']
  (4) ['load_image', 'load_mask', 'load_poni', 'process']
  (2) ['status', 'Symbol(Symbol.dispose)']
[init] Setting up UI event listeners...
[setupUploadZones] Found 4 upload zones
[setupUploadZones] Setting up zone 0: poni
[setupUploadZones] Configured input element
[setupUploadZones] Zone poni configured
[setupUploadZones] Setting up zone 1: bright_field
[setupUploadZones] Configured input element
[setupUploadZones] Zone bright_field configured
[setupUploadZones] Setting up zone 2: mask
[setupUploadZones] Configured input element
[setupUploadZones] Zone mask configured
[setupUploadZones] Setting up zone 3: image
[setupUploadZones] Configured input element
[setupUploadZones] Zone image configured
[setupUploadZones] All zones configured
=== ✓ UI initialized successfully ===
```

If you see this, the app is ready! ✓

---

## Expected Upload Sequence (Click or Drag a PONI file)

```
[setupUploadZones] File input changed for poni: FileList {0: File, length: 1}
[handleFileUpload] Starting upload for poni: calibration.poni
[handleFileUpload] appState.explorer is not null ✓
[handleFileUpload] File read complete, size: 477 bytes
[handleFileUpload] Processing poni
[handleFileUpload] Decoding PONI as text
[handleFileUpload] Calling load_poni with 477 chars
[handleFileUpload] Result: {status: 'success', message: 'PONI loaded', ...}
[handleFileUpload] poni loaded successfully
```

Then you should see a **green success message** on the page:
```
✓ calibration.poni loaded successfully
```

And the status bar should update to show "✓" next to "PONI".

---

## Expected NPY File Upload (Bright Field)

```
[setupUploadZones] File input changed for bright_field: FileList {0: File, length: 1}
[handleFileUpload] Starting upload for bright_field: bright_field.npy
[handleFileUpload] appState.explorer is not null ✓
[handleFileUpload] File read complete, size: 17895424 bytes
[handleFileUpload] Processing bright_field
[handleFileUpload] Encoding bright_field as base64
[handleFileUpload] Calling load_bright_field with 23860568 bytes (b64)
[handleFileUpload] Result: {status: 'success', message: 'Bright field loaded', ...}
[handleFileUpload] bright_field loaded successfully
```

---

## If Something Goes Wrong

### Missing WASM Module
```
[init] Trying WASM path: ./pkg/pilatus4_explorer.js
[init] Failed to load from ./pkg/pilatus4_explorer.js: 
  Failed to fetch
Failed to load WASM module from any path. 
  Last error: Failed to fetch
```

**Fix:** Check that `/Users/openclaw/.openclaw/workspace/projects/pilatus4_explorer/pkg/` exists

### WASM Module Not Initialized
```
[handleFileUpload] Starting upload for poni: calibration.poni
[handleFileUpload] appState.explorer is null ❌
WASM module not initialized. Please refresh the page.
```

**Fix:** The app tried to initialize and failed. Check earlier logs for errors.

### File Not Readable
```
[handleFileUpload] File read complete, size: 0 bytes
[handleFileUpload] Processing poni
[handleFileUpload] Error processing poni: SyntaxError: Unexpected end of JSON input
```

**Fix:** The file might be corrupted or the wrong type. Try downloading it again.

---

## How to Copy Logs from Console

1. Right-click in the console
2. Select "Save as..." to download console history
3. Or select all text (Cmd+A / Ctrl+A) and copy (Cmd+C / Ctrl+C)
4. Paste into a text editor if you need to share

---

**Last Updated:** 2026-04-28  
**WASM Initialization Fix:** ✅ Applied
