# Debug Guide for PILATUS4 Data Explorer

## If File Loading Fails Silently

The issue has been fixed with proper WASM module initialization. If you're still experiencing problems:

### Step 1: Open Browser Console

1. **Chrome/Edge/Firefox:** Press `Ctrl+Shift+J` (Windows) or `Cmd+Shift+J` (Mac)
2. **Safari:** Press `Cmd+Option+I`, then click "Console" tab
3. You should see detailed logs starting with `=== Initializing PILATUS4 Data Explorer ===`

### Step 2: Check for These Log Messages (in order)

✓ **Good startup sequence:**
```
=== Initializing PILATUS4 Data Explorer ===
Current location: file:///Users/openclaw/.openclaw/workspace/projects/pilatus4_explorer/index.html
[init] Trying WASM path: ./pkg/pilatus4_explorer.js
✓ WASM module loaded from: ./pkg/pilatus4_explorer.js
[init] Initializing WASM module...
✓ WASM module initialized
[init] Creating DataExplorer instance...
✓ DataExplorer instance created
[init] Available methods: [list of methods...]
[setupUploadZones] Found 4 upload zones
[setupUploadZones] Setting up zone poni
...
=== ✓ UI initialized successfully ===
```

✗ **If you see errors:**
- Look for red text in the console (errors)
- Check the exact error message
- Look for stack traces

### Step 3: Try Uploading a File

When you click or drag a file:

```
[setupUploadZones] Clicked on poni
[setupUploadZones] File input changed for poni: FileList { 0: File ... }
[handleFileUpload] Starting upload for poni: calibration.poni
[handleFileUpload] appState.explorer is null  ← This would be the bug
```

### Step 4: Common Issues & Fixes

| Issue | Message | Fix |
|-------|---------|-----|
| WASM not loading | `Failed to load WASM module` | Check `pkg/` directory exists and has `.wasm` file |
| Module path wrong | `Trying WASM path: ./pkg/...` (tries all 3) | Ensure file is served from correct path |
| WASM not initialized | Explorer instance appears empty | Already fixed in latest version |
| File reader fails | `[handleFileUpload] FileReader error` | File might be too large or corrupted |
| WASM method fails | `[handleFileUpload] Inner error for...` | Check Rust error message in output |

### Step 5: Test with Console Commands

In the browser console, you can test directly:

```javascript
// Check if explorer is initialized
console.log(appState.explorer);

// Check detector info
console.log(appState.explorer.detector_info());

// Check loaded files
console.log(appState.loadedFiles);
```

## Manual Testing Without UI

If the UI fails to initialize, you can still test WASM directly:

```javascript
// In browser console:
import('./pkg/pilatus4_explorer.js').then(async (m) => {
  await m.default();  // Initialize WASM
  const explorer = new m.DataExplorer();
  console.log(explorer.detector_info());
});
```

## Rebuilding if Everything Else Fails

```bash
cd /Users/openclaw/.openclaw/workspace/projects/pilatus4_explorer

# Clean rebuild
rm -rf pkg/
wasm-pack build --target web --release

# Then reload the page in browser
```

---

**Note:** All changes made on 2026-04-28 to fix WASM initialization. If the page was cached, do a **hard refresh** (Cmd+Shift+R on Mac, Ctrl+Shift+R on Windows).
