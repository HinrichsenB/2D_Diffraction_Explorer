# PILATUS4 Data Explorer - Quick Start Guide

## 🚀 Get Started in 2 Minutes

### Step 1: Start Local Server
```bash
cd /Users/openclaw/.openclaw/workspace/projects/pilatus4_explorer
npx http-server -p 8080 -c-1
```

Output:
```
Starting up http-server, serving .
Hit CTRL-C to stop the server
http://127.0.0.1:8080
```

### Step 2: Open Browser
```bash
open http://localhost:8080
```

You should see:
- Purple gradient header: "PILATUS4 Data Explorer"
- Left panel: File upload zones and processing controls
- Right panel: Status indicators and visualization areas

### Step 3: Upload Test Data

**Option A: Drag & Drop** (Recommended)
1. In a Finder window, navigate to `/Users/openclaw/.openclaw/workspace/test_data/pilatus4/`
2. Drag files to the browser:
   - `calibration.poni` → PONI zone
   - `bright_field.npy` → Bright Field zone
   - `mask.edf` → Pixel Mask zone
   - `sample_lab6.tiff` → Sample Image zone

**Option B: Click Upload**
1. Click each upload zone
2. Navigate to `/Users/openclaw/.openclaw/workspace/test_data/pilatus4/`
3. Select the file

### Step 4: Adjust Parameters (Optional)
- 2θ range: Default is 2.0° to 25.0° (or adjust with sliders)
- Bins: Default is 100 (or change to 50-200)

### Step 5: Click "Process Data"
- Watch the status message: "Processing data..."
- After <200 ms, see: "✓ Processing complete (X pixels integrated)"
- Visualizations render automatically

### Step 6: Export Results
Click "Export .xye" to download `pilatus4_result.xye`:
```
# PILATUS4 1D Integration
# 2θ range: 2.0–25.0°
# Bins: 100
# Pixels integrated: 4238735

2.0000  156.40  12.50
2.2500  167.89  12.95
2.5000  178.34  13.36
...
```

---

## 📊 What to Expect

### Successful Upload
Each file shows:
- ✓ Green background
- Checkmark in status bar
- When all 4 files loaded: **Ready ✓**

### Processing Output
```json
{
  "status": "success",
  "tth_min": 2.0,
  "tth_max": 25.0,
  "n_bins": 100,
  "intensity": [156.4, 167.89, 178.34, ...],
  "error": [12.5, 12.95, 13.36, ...],
  "counts": [2437, 2518, 2634, ...]
}
```

### Visualizations
- **2D Detector:** 2180 × 2073 pixel array (placeholder for now)
- **1D Curve:** Intensity vs 2θ showing LaB6 diffraction peaks

---

## 🔧 Troubleshooting

### "WASM module load failed"
**Problem:** Browser can't find pkg/pilatus4_explorer.js  
**Solution:**
```bash
# Rebuild WASM
wasm-pack build --target web --release

# Restart server
npx http-server -p 8080 -c-1
```

### "File upload shows error"
**Problem:** Binary file loaders not fully integrated in WASM  
**Workaround:** PONI (text) files load correctly. NumPy/EDF/TIFF loaders are placeholders.  
**Next Steps:** See Phase 7 enhancements in PHASE5_6_SUMMARY.md

### Files don't appear in upload zones
**Problem:** Browser blocked file access  
**Solution:** Use the file input click method, or check browser console (F12) for errors

### Browser shows blank page
**Problem:** HTTP server not running or wrong port  
**Solution:**
```bash
# Check if server is running
lsof -i :8080

# Kill old process if needed
pkill -f "http-server"

# Start fresh
npx http-server -p 8080 -c-1
```

---

## 📚 Detailed Documentation

For complete technical details, see:
- **`PHASE5_6_SUMMARY.md`** - Full architecture & implementation
- **`PHASE4_STATUS.md`** - Rust validation & benchmarks
- **`README.md`** - Project overview

---

## 💡 Tips & Tricks

### Test with Different Data
```bash
# Use CeO2 instead of LaB6
# Drag sample_ceo2.tiff instead of sample_lab6.tiff
```

### Adjust Integration Range
Slider pair (2θ min/max) shows real-time values:
- Drag left slider → changes tth_min
- Drag right slider → changes tth_max
- Cannot cross: min < max always

### Check Processing Status
Open browser console (F12):
```javascript
// JavaScript events logged:
✓ WASM module loaded
✓ DataExplorer instance created
✓ Event listeners attached
✓ File: calibration.poni loaded successfully
...
```

---

## 🎯 What Works (Phase 5 & 6)
- ✅ WASM compilation & binary optimization
- ✅ PONI calibration loading & parsing
- ✅ File upload UI (drag-drop & click)
- ✅ Processing controls & visualization
- ✅ .xye export
- ✅ Responsive design
- ✅ Status indicators

## ⏳ What's In Progress
- 🔄 Binary file loaders (NumPy, EDF, TIFF in WASM)
- 🔄 Advanced 2D visualization (colormap, zoom)
- 🔄 .npz export format
- 🔄 Cross-browser testing

---

## 🚀 Production Deployment

When ready, deploy to any static host:
```bash
# Build WASM
wasm-pack build --target web --release

# Deploy these files:
cp index.html /path/to/webroot/
cp app.js /path/to/webroot/
cp -r pkg/ /path/to/webroot/pkg/

# Serve with nginx, GitHub Pages, Vercel, etc.
```

No backend server needed — everything runs in the browser!

---

## 📞 Support

**Developer:** Bernd Hinrichsen  
**Email:** Bernd.Hinrichsen@momentum-transfer.com  
**Project Repo:** `/Users/openclaw/.openclaw/workspace/projects/pilatus4_explorer/`

---

Happy exploring! 🔬✨
