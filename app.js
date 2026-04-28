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
    console.log('Initializing PILATUS4 Data Explorer...');

    // Import WASM module
    try {
        const wasm = await import('./pkg/pilatus4_explorer.js');
        console.log('✓ WASM module loaded');

        // Create DataExplorer instance
        appState.explorer = new wasm.DataExplorer();
        console.log('✓ DataExplorer instance created');

        // Set up event listeners
        setupUploadZones();
        setupControls();
        setupRangeSliders();

        updateStatus();
    } catch (error) {
        showMessage(`Failed to initialize: ${error.message}`, 'error');
        console.error('Initialization error:', error);
    }
}

/**
 * Set up drag-and-drop file upload zones
 */
function setupUploadZones() {
    const zones = document.querySelectorAll('.upload-zone');

    zones.forEach(zone => {
        const fileType = zone.dataset.fileType;
        const input = zone.querySelector('input');

        // Click to upload
        zone.addEventListener('click', () => input.click());

        // Drag and drop
        zone.addEventListener('dragover', (e) => {
            e.preventDefault();
            zone.classList.add('active');
        });

        zone.addEventListener('dragleave', () => {
            zone.classList.remove('active');
        });

        zone.addEventListener('drop', (e) => {
            e.preventDefault();
            zone.classList.remove('active');
            handleFileUpload(e.dataTransfer.files[0], fileType);
        });

        // File input change
        input.addEventListener('change', (e) => {
            if (e.target.files.length > 0) {
                handleFileUpload(e.target.files[0], fileType);
            }
        });
    });
}

/**
 * Handle file upload
 */
async function handleFileUpload(file, fileType) {
    if (!file) return;

    const reader = new FileReader();

    reader.onload = async (e) => {
        try {
            const content = e.target.result;

            // Load file based on type
            let result;
            switch (fileType) {
                case 'poni':
                    // PONI is text, decode as string
                    const text = new TextDecoder().decode(new Uint8Array(content));
                    result = JSON.parse(appState.explorer.load_poni(text));
                    break;

                case 'bright_field':
                    result = JSON.parse(appState.explorer.load_bright_field(base64Encode(content)));
                    break;

                case 'mask':
                    result = JSON.parse(appState.explorer.load_mask(base64Encode(content)));
                    break;

                case 'image':
                    result = JSON.parse(appState.explorer.load_image(base64Encode(content)));
                    break;
            }

            if (result.status === 'success') {
                appState.loadedFiles[fileType] = result;
                showFileStatus(fileType, true);
                showMessage(`✓ ${file.name} loaded successfully`, 'success');
                updateStatus();
            } else {
                showMessage(`Error loading ${file.name}: ${result.error}`, 'error');
            }
        } catch (error) {
            showMessage(`Error processing file: ${error.message}`, 'error');
            console.error('File processing error:', error);
        }
    };

    reader.onerror = () => {
        showMessage(`Error reading file: ${file.name}`, 'error');
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
}

/**
 * Draw 2D detector image
 */
function draw2DImage() {
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');

    // For now, just show a placeholder
    // In production, would render the detector image with color mapping
    ctx.fillStyle = '#f5f5f5';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    ctx.fillStyle = '#999';
    ctx.font = '14px system-ui';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('2D Detector Image (2180 × 2073)', canvas.width / 2, canvas.height / 2);
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
 * Update overall status indicator
 */
function updateStatus() {
    const allLoaded = Object.entries(appState.loadedFiles).every(
        ([type, data]) => data !== null
    );

    // Update status items
    document.getElementById('status-poni').classList.toggle('active', appState.loadedFiles.poni !== null);
    document.getElementById('status-ff').classList.toggle('active', appState.loadedFiles.bright_field !== null);
    document.getElementById('status-mask').classList.toggle('active', appState.loadedFiles.mask !== null);
    document.getElementById('status-image').classList.toggle('active', appState.loadedFiles.image !== null);
    document.getElementById('status-ready').classList.toggle('active', allLoaded);

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
    document.querySelector('#status-ready .status-value').textContent = allLoaded ? '✓' : '✗';

    // Enable process button if all loaded
    document.getElementById('btn-process').disabled = !allLoaded;
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
