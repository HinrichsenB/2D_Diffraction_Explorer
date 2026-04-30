/**
 * PILATUS4 Data Explorer - Phase 6 JavaScript UI
 * 
 * Handles file uploads, WASM module initialization, data processing,
 * visualization, and export functionality.
 */

// Global state
const appState = {
    explorer: null,
    loadedFiles: {
        poni: null,
        bright_field: null,
        mask: null,
        image: null,
    },
    lastResult: null,
    isProcessing: false,
};

/**
 * Initialize the application
 */
async function init() {
    console.log('=== Initializing PILATUS4 Data Explorer ===');
    console.log('Current location:', window.location.href);
    console.log('Script directory:', import.meta.url);

    // Import WASM module - try relative path first (most likely)
    let wasm = null;
    const paths = ['./pkg/pilatus4_explorer.js', '/pkg/pilatus4_explorer.js', '/2D_Diffraction_Explorer/pkg/pilatus4_explorer.js'];
    let lastError = null;

    for (const path of paths) {
        try {
            console.log(`[init] Trying WASM path: ${path}`);
            wasm = await import(path);
            console.log(`✓ WASM module loaded from: ${path}`);
            break;
        } catch (error) {
            console.warn(`[init] Failed to load from ${path}:`, error.message);
            lastError = error;
        }
    }

    if (!wasm) {
        const msg = `Failed to load WASM module from any path. Last error: ${lastError?.message}`;
        showMessage(msg, 'error');
        console.error('[init]', msg);
        console.error('[init] Last error:', lastError);
        return;
    }

    try {
        // Initialize WASM module (critical step!)
        console.log('[init] Initializing WASM module...');
        const wasmInit = wasm.default || wasm.__wbg_init;
        if (typeof wasmInit !== 'function') {
            throw new Error('WASM default export is not a function. Available exports:', Object.keys(wasm));
        }
        await wasmInit();
        console.log('[init] ✓ WASM module initialized');

        // Create DataExplorer instance
        console.log('[init] Creating DataExplorer instance...');
        appState.explorer = new wasm.DataExplorer();
        console.log('[init] ✓ DataExplorer instance created');
        console.log('[init] Available methods:', Object.getOwnPropertyNames(Object.getPrototypeOf(appState.explorer)));

        // Set up event listeners
        console.log('[init] Setting up UI event listeners...');
        setupUploadZones();
        setupControls();
        setupRangeSliders();

        updateStatus();
        console.log('=== ✓ UI initialized successfully ===');
    } catch (error) {
        showMessage(`Failed to initialize UI: ${error.message}`, 'error');
        console.error('[init] Initialization error:', error);
        console.error('[init] Error stack:', error.stack);
    }
}

/**
 * Set up drag-and-drop file upload zones
 */
function setupUploadZones() {
    const zones = document.querySelectorAll('.upload-zone');
    console.log(`[setupUploadZones] Found ${zones.length} upload zones`);

    zones.forEach((zone, idx) => {
        const fileType = zone.dataset.fileType;
        const input = zone.querySelector('input');
        
        console.log(`[setupUploadZones] Setting up zone ${idx}: ${fileType}`);

        if (!input) {
            console.error(`[setupUploadZones] No input element found for zone ${fileType}`);
            return;
        }

        // Click to upload
        zone.addEventListener('click', () => {
            console.log(`[setupUploadZones] Clicked on ${fileType}`);
            input.click();
        });

        // Drag and drop
        zone.addEventListener('dragover', (e) => {
            e.preventDefault();
            zone.classList.add('active');
            console.log(`[setupUploadZones] Dragover on ${fileType}`);
        });

        zone.addEventListener('dragleave', () => {
            zone.classList.remove('active');
        });

        zone.addEventListener('drop', (e) => {
            e.preventDefault();
            zone.classList.remove('active');
            console.log(`[setupUploadZones] Dropped file(s) on ${fileType}`);
            handleFileUpload(e.dataTransfer.files[0], fileType);
        });

        // File input change
        input.addEventListener('change', (e) => {
            console.log(`[setupUploadZones] File input changed for ${fileType}:`, e.target.files);
            if (e.target.files.length > 0) {
                handleFileUpload(e.target.files[0], fileType);
            }
        });
        
        console.log(`[setupUploadZones] ✓ Zone ${fileType} configured`);
    });
    
    console.log('[setupUploadZones] All zones configured');
}

/**
 * Handle file upload
 */
async function handleFileUpload(file, fileType) {
    if (!file) return;

    console.log(`[handleFileUpload] Starting upload for ${fileType}:`, file.name);

    // Check if WASM is ready
    if (!appState.explorer) {
        showMessage('WASM module not initialized. Please refresh the page.', 'error');
        console.error('[handleFileUpload] appState.explorer is null');
        return;
    }

    const reader = new FileReader();

    reader.onload = async (e) => {
        try {
            const content = e.target.result;
            console.log(`[handleFileUpload] File read complete, size: ${content.byteLength} bytes`);

            // Load file based on type
            let result;
            console.log(`[handleFileUpload] Processing ${fileType}`);
            
            try {
                switch (fileType) {
                    case 'poni':
                        // PONI is text, decode as string
                        console.log('[handleFileUpload] Decoding PONI as text');
                        const text = new TextDecoder().decode(new Uint8Array(content));
                        console.log('[handleFileUpload] Calling load_poni with', text.length, 'chars');
                        result = JSON.parse(appState.explorer.load_poni(text));
                        break;

                    case 'bright_field':
                        console.log('[handleFileUpload] Encoding bright_field as base64');
                        const bf_b64 = base64Encode(content);
                        console.log('[handleFileUpload] Calling load_bright_field with', bf_b64.length, 'bytes (b64)');
                        result = JSON.parse(appState.explorer.load_bright_field(bf_b64));
                        break;

                    case 'mask':
                        console.log('[handleFileUpload] Encoding mask as base64');
                        const mask_b64 = base64Encode(content);
                        console.log('[handleFileUpload] Calling load_mask with', mask_b64.length, 'bytes (b64)');
                        result = JSON.parse(appState.explorer.load_mask(mask_b64));
                        break;

                    case 'image':
                        console.log('[handleFileUpload] Encoding image as base64');
                        const img_b64 = base64Encode(content);
                        console.log('[handleFileUpload] Calling load_image with', img_b64.length, 'bytes (b64)');
                        result = JSON.parse(appState.explorer.load_image(img_b64));
                        break;

                    default:
                        throw new Error(`Unknown file type: ${fileType}`);
                }

                console.log(`[handleFileUpload] Result:`, result);

                if (result && result.status === 'success') {
                    appState.loadedFiles[fileType] = result;
                    showFileStatus(fileType, true);
                    showMessage(`✓ ${file.name} loaded successfully`, 'success');
                    updateStatus();
                    console.log(`[handleFileUpload] ${fileType} loaded successfully`);
                } else if (result && result.error) {
                    showMessage(`Error loading ${file.name}: ${result.error}`, 'error');
                    console.error(`[handleFileUpload] Error from WASM: ${result.error}`);
                } else {
                    showMessage(`Unexpected response format for ${file.name}`, 'error');
                    console.error(`[handleFileUpload] Unexpected result:`, result);
                }
            } catch (innerError) {
                showMessage(`Error processing ${fileType}: ${innerError.message}`, 'error');
                console.error(`[handleFileUpload] Inner error for ${fileType}:`, innerError);
                throw innerError;
            }
        } catch (error) {
            showMessage(`Error processing file: ${error.message}`, 'error');
            console.error('[handleFileUpload] Outer error:', error);
        }
    };

    reader.onerror = () => {
        showMessage(`Error reading file: ${file.name}`, 'error');
        console.error(`[handleFileUpload] FileReader error for ${file.name}:`, reader.error);
    };

    // Read as ArrayBuffer
    reader.readAsArrayBuffer(file);
}

/**
 * Show file status indicator
 */
function showFileStatus(fileType, isLoaded) {
    const item = document.getElementById(`file-${fileType}`);
    if (item) {
        item.style.display = isLoaded ? 'flex' : 'none';
        item.classList.toggle('loaded', isLoaded);
    }
}

/**
 * Set up processing controls
 */
function setupControls() {
    document.getElementById('btn-process').addEventListener('click', processData);
    document.getElementById('btn-export-xye').addEventListener('click', exportXYE);
    document.getElementById('btn-export-npz').addEventListener('click', exportNPZ);
}

/**
 * Set up range slider displays
 */
function setupRangeSliders() {
    const minSlider = document.getElementById('tth-min');
    const maxSlider = document.getElementById('tth-max');
    const minDisplay = document.getElementById('tth-min-display');
    const maxDisplay = document.getElementById('tth-max-display');

    minSlider.addEventListener('input', () => {
        minDisplay.textContent = minSlider.value;
        // Ensure min < max
        if (parseFloat(minSlider.value) > parseFloat(maxSlider.value)) {
            maxSlider.value = minSlider.value;
            maxDisplay.textContent = maxSlider.value;
        }
    });

    maxSlider.addEventListener('input', () => {
        maxDisplay.textContent = maxSlider.value;
        // Ensure max > min
        if (parseFloat(maxSlider.value) < parseFloat(minSlider.value)) {
            minSlider.value = maxSlider.value;
            minDisplay.textContent = minSlider.value;
        }
    });
}

/**
 * Process the loaded data
 */
async function processData() {
    if (appState.isProcessing) return;

    const allLoaded = Object.values(appState.loadedFiles).every(v => v !== null);
    if (!allLoaded) {
        showMessage('Please load all required files first', 'error');
        return;
    }

    appState.isProcessing = true;
    document.getElementById('btn-process').disabled = true;

    try {
        const tthMin = parseFloat(document.getElementById('tth-min').value);
        const tthMax = parseFloat(document.getElementById('tth-max').value);
        const nBins = parseInt(document.getElementById('n-bins').value);

        showMessage('Processing data...', 'info');

        // Call WASM processing
        const resultStr = appState.explorer.process(tthMin, tthMax, nBins);
        const result = JSON.parse(resultStr);

        if (result.status === 'success') {
            appState.lastResult = result;
            showMessage(`✓ Processing complete (${result.n_pixels_integrated} pixels)`, 'success');

            // Update visualizations
            updateVisualizations(result);

            // Enable export buttons
            document.getElementById('btn-export-xye').disabled = false;
            document.getElementById('btn-export-npz').disabled = false;
        } else {
            showMessage(`Processing error: ${result.error}`, 'error');
        }
    } catch (error) {
        showMessage(`Processing failed: ${error.message}`, 'error');
        console.error('Processing error:', error);
    } finally {
        appState.isProcessing = false;
        document.getElementById('btn-process').disabled = false;
    }
}

/**
 * Update visualizations with results
 */
function updateVisualizations(result) {
    // Draw 2D detector image
    draw2DImage();

    // Draw 1D integration curve
    draw1DCurve(result);

    // Draw LUT geometry (debugging)
    drawLUTGeometry();
}

/**
 * Draw 2D detector image
 */
function draw2DImage() {
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');

    // Get image data from WASM
    if (!appState.explorer) {
        ctx.fillStyle = 'white';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.fillStyle = '#999';
        ctx.font = '12px system-ui';
        ctx.fillText('WASM not initialized', 10, 20);
        return;
    }

    try {
        const imageDataStr = appState.explorer.get_image_data();
        const imageData = JSON.parse(imageDataStr);

        if (imageData.status !== 'success') {
            ctx.fillStyle = '#f5f5f5';
            ctx.fillRect(0, 0, canvas.width, canvas.height);
            ctx.fillStyle = '#999';
            ctx.font = '12px system-ui';
            ctx.textAlign = 'center';
            ctx.fillText('Error loading image data', canvas.width / 2, canvas.height / 2);
            return;
        }

        const width = imageData.width;
        const height = imageData.height;
        const minVal = imageData.min_val;
        const maxVal = imageData.max_val;

        // Decode RGBA data from base64
        const rgbaBase64 = imageData.rgba_base64;
        const binaryString = atob(rgbaBase64);
        const bytes = new Uint8ClampedArray(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
            bytes[i] = binaryString.charCodeAt(i);
        }

        // Create ImageData and display
        const imgData = ctx.createImageData(width, height);
        imgData.data.set(bytes);

        // Calculate display dimensions to fit in canvas
        const aspectRatio = width / height;
        let displayWidth = canvas.width;
        let displayHeight = canvas.width / aspectRatio;

        if (displayHeight > canvas.height) {
            displayHeight = canvas.height;
            displayWidth = canvas.height * aspectRatio;
        }

        const offsetX = (canvas.width - displayWidth) / 2;
        const offsetY = (canvas.height - displayHeight) / 2;

        // Draw on temporary canvas then scale to main canvas
        const tempCanvas = document.createElement('canvas');
        tempCanvas.width = width;
        tempCanvas.height = height;
        const tempCtx = tempCanvas.getContext('2d');
        tempCtx.putImageData(imgData, 0, 0);

        ctx.fillStyle = 'white';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(tempCanvas, offsetX, offsetY, displayWidth, displayHeight);

        // Add colorbar info
        ctx.fillStyle = '#333';
        ctx.font = '11px system-ui';
        ctx.textAlign = 'left';
        ctx.fillText(`Min: ${minVal}`, 10, canvas.height - 5);
        ctx.textAlign = 'right';
        ctx.fillText(`Max: ${maxVal}`, canvas.width - 10, canvas.height - 5);
    } catch (error) {
        console.error('[draw2DImage] Error:', error);
        ctx.fillStyle = '#f5f5f5';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.fillStyle = '#c33';
        ctx.font = '12px system-ui';
        ctx.textAlign = 'center';
        ctx.fillText('Error: ' + error.message, canvas.width / 2, canvas.height / 2);
    }
}

/**
 * Draw 1D integration curve
 */
function draw1DCurve(result) {
    const canvas = document.getElementById('graph');
    const ctx = canvas.getContext('2d');

    if (!result.intensity || result.intensity.length === 0) {
        ctx.fillStyle = '#999';
        ctx.font = '14px system-ui';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('No data to plot', canvas.width / 2, canvas.height / 2);
        return;
    }

    // Simple plot rendering
    const padding = 40;
    const width = canvas.width - 2 * padding;
    const height = canvas.height - 2 * padding;

    // Clear canvas
    ctx.fillStyle = 'white';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Draw axes
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(padding, canvas.height - padding);
    ctx.lineTo(canvas.width - padding, canvas.height - padding);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(padding, padding);
    ctx.lineTo(padding, canvas.height - padding);
    ctx.stroke();

    // Find min/max for scaling
    const intensity = result.intensity;
    const maxIntensity = Math.max(...intensity);
    const minIntensity = Math.min(...intensity);
    const range = maxIntensity - minIntensity || 1;

    // Draw curve
    ctx.strokeStyle = '#667eea';
    ctx.lineWidth = 2;
    ctx.beginPath();

    intensity.forEach((value, i) => {
        const x = padding + (i / (intensity.length - 1)) * width;
        const y = canvas.height - padding - ((value - minIntensity) / range) * height;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    });

    ctx.stroke();

    // Draw labels
    ctx.fillStyle = '#666';
    ctx.font = '12px system-ui';
    ctx.textAlign = 'center';
    ctx.fillText('2θ (degrees)', canvas.width / 2, canvas.height - 10);

    ctx.save();
    ctx.translate(10, canvas.height / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('Intensity (counts)', 0, 0);
    ctx.restore();
}

/**
 * Draw LUT (Look-Up Table) geometry visualization
 */
function drawLUTGeometry() {
    const canvas = document.getElementById('lut-canvas');
    const ctx = canvas.getContext('2d');

    if (!appState.explorer) {
        ctx.fillStyle = 'white';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.fillStyle = '#999';
        ctx.font = '12px system-ui';
        ctx.fillText('WASM not initialized', 10, 20);
        return;
    }

    try {
        const lutStr = appState.explorer.get_lut();
        const lut = JSON.parse(lutStr);

        if (lut.status !== 'success') {
            ctx.fillStyle = 'white';
            ctx.fillRect(0, 0, canvas.width, canvas.height);
            ctx.fillStyle = '#999';
            ctx.font = '12px system-ui';
            ctx.textAlign = 'center';
            ctx.fillText('LUT Error', canvas.width / 2, canvas.height / 2);
            return;
        }

        // Clear canvas
        ctx.fillStyle = 'white';
        ctx.fillRect(0, 0, canvas.width, canvas.height);

        // Title and info
        ctx.fillStyle = '#333';
        ctx.font = 'bold 12px system-ui';
        ctx.textAlign = 'left';
        ctx.fillText('LUT Geometry Sample (Distance: ' + lut.distance_m.toFixed(3) + ' m)', 10, 20);

        // Draw 2θ vs χ scatter
        const padding = 40;
        const width = canvas.width - padding - 10;
        const height = canvas.height - padding - 10;

        // Find min/max for axes
        let minTTH = Infinity, maxTTH = -Infinity;
        let minChi = Infinity, maxChi = -Infinity;

        lut.lut_samples.forEach(sample => {
            minTTH = Math.min(minTTH, sample.two_theta_deg);
            maxTTH = Math.max(maxTTH, sample.two_theta_deg);
            minChi = Math.min(minChi, sample.chi_deg);
            maxChi = Math.max(maxChi, sample.chi_deg);
        });

        // Add margins
        const tthRange = (maxTTH - minTTH) || 1;
        const chiRange = (maxChi - minChi) || 1;
        minTTH -= tthRange * 0.05;
        maxTTH += tthRange * 0.05;
        minChi -= chiRange * 0.05;
        maxChi += chiRange * 0.05;

        // Draw axes
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(padding, canvas.height - padding);
        ctx.lineTo(canvas.width - 10, canvas.height - padding);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(padding, padding + 10);
        ctx.lineTo(padding, canvas.height - padding);
        ctx.stroke();

        // Plot points
        ctx.fillStyle = '#667eea';
        ctx.globalAlpha = 0.6;
        lut.lut_samples.forEach(sample => {
            const x = padding + ((sample.two_theta_deg - minTTH) / (maxTTH - minTTH)) * width;
            const y = canvas.height - padding - ((sample.chi_deg - minChi) / (maxChi - minChi)) * height;
            ctx.fillRect(x - 2, y - 2, 4, 4);
        });
        ctx.globalAlpha = 1.0;

        // Labels
        ctx.fillStyle = '#333';
        ctx.font = '10px system-ui';
        ctx.textAlign = 'center';
        ctx.fillText('2θ (deg)', canvas.width / 2, canvas.height - 5);
        ctx.save();
        ctx.translate(5, canvas.height / 2);
        ctx.rotate(-Math.PI / 2);
        ctx.textAlign = 'center';
        ctx.fillText('χ (deg)', 0, 0);
        ctx.restore();
    } catch (error) {
        console.error('[drawLUTGeometry] Error:', error);
        ctx.fillStyle = 'white';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.fillStyle = '#c33';
        ctx.font = '10px system-ui';
        ctx.textAlign = 'center';
        ctx.fillText('Error: ' + error.message, canvas.width / 2, canvas.height / 2);
    }
}

/**
 * Update overall status indicator
 */
function updateStatus() {
    // Only PONI and Image are required
    const minRequired = appState.loadedFiles.poni !== null && appState.loadedFiles.image !== null;

    // Update status items
    document.getElementById('status-poni').classList.toggle('active', appState.loadedFiles.poni !== null);
    document.getElementById('status-ff').classList.toggle('active', appState.loadedFiles.bright_field !== null);
    document.getElementById('status-mask').classList.toggle('active', appState.loadedFiles.mask !== null);
    document.getElementById('status-image').classList.toggle('active', appState.loadedFiles.image !== null);
    document.getElementById('status-ready').classList.toggle('active', minRequired);

    // Update status values
    if (appState.loadedFiles.poni) {
        document.querySelector('#status-poni .status-value').textContent = '✓';
    }
    if (appState.loadedFiles.bright_field) {
        document.querySelector('#status-ff .status-value').textContent = '✓';
    }
    if (appState.loadedFiles.mask) {
        document.querySelector('#status-mask .status-value').textContent = '✓';
    }
    if (appState.loadedFiles.image) {
        document.querySelector('#status-image .status-value').textContent = '✓';
    }
    document.querySelector('#status-ready .status-value').textContent = minRequired ? '✓' : '✗';

    // Enable process button if minimum required files are loaded
    document.getElementById('btn-process').disabled = !minRequired;
}

/**
 * Export results as .xye format
 */
function exportXYE() {
    if (!appState.lastResult) return;

    const result = appState.lastResult;
    let content = '# PILATUS4 1D Integration\n';
    content += `# 2θ range: ${result.tth_min}–${result.tth_max}°\n`;
    content += `# Bins: ${result.n_bins}\n`;
    content += `# Pixels integrated: ${result.n_pixels_integrated}\n`;
    content += '\n';

    if (result.intensity && result.error) {
        const nBins = result.intensity.length;
        const tthStep = (result.tth_max - result.tth_min) / nBins;

        for (let i = 0; i < nBins; i++) {
            const tth = result.tth_min + i * tthStep;
            const intensity = result.intensity[i];
            const error = result.error[i];
            content += `${tth.toFixed(4)}  ${intensity.toFixed(2)}  ${error.toFixed(2)}\n`;
        }
    }

    downloadFile(content, 'pilatus4_result.xye', 'text/plain');
}

/**
 * Export results as .npz format (requires additional library)
 */
function exportNPZ() {
    if (!appState.lastResult) return;

    showMessage('NPZ export requires numpy.js library (not yet implemented)', 'info');
    // TODO: Implement NPZ export with a library like numpy.js or custom implementation
}

/**
 * Show message to user
 */
function showMessage(text, type = 'info') {
    const msg = document.getElementById('status-message');
    msg.textContent = text;
    msg.className = `message show ${type}`;

    if (type !== 'error') {
        setTimeout(() => {
            msg.classList.remove('show');
        }, 5000);
    }
}

/**
 * Helper: Convert ArrayBuffer to base64 string
 */
function base64Encode(arrayBuffer) {
    const bytes = new Uint8Array(arrayBuffer);
    let binary = '';
    for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return btoa(binary);
}

/**
 * Helper: Download file
 */
function downloadFile(content, filename, mimeType) {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

// Initialize on page load
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
