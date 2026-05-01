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

    // Draw 3D LUT geometry
    draw3DLUTGeometry();
}

/**
 * Draw 2D detector image with 99.5 percentile scaling
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
        // Try to get raw data first (new method), fallback to old method
        let imageData = null;
        try {
            const rawDataStr = appState.explorer.get_image_raw();
            imageData = JSON.parse(rawDataStr);
        } catch (e) {
            console.warn('[draw2DImage] Raw data method not available, using old method');
            const imageDataStr = appState.explorer.get_image_data();
            imageData = JSON.parse(imageDataStr);
        }

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
        
        // Get raw data with 99.5 percentile
        let rawData = null;
        let minVal = imageData.min_val || 0;
        let p995 = imageData.p995_val || imageData.max_val || 1;
        
        if (imageData.raw_data_b64) {
            // New method: decode base64 raw data
            const binaryString = atob(imageData.raw_data_b64);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }
            const view = new DataView(bytes.buffer);
            rawData = [];
            for (let i = 0; i < bytes.length; i += 4) {
                rawData.push(view.getFloat32(i, true)); // true = little-endian
            }
        } else if (imageData.rgba_base64) {
            // Fallback: use pre-rendered RGBA data
            const binaryString = atob(imageData.rgba_base64);
            const bytes = new Uint8ClampedArray(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }
            const imgData = ctx.createImageData(width, height);
            imgData.data.set(bytes);
            
            const aspectRatio = width / height;
            let displayWidth = canvas.width;
            let displayHeight = canvas.width / aspectRatio;
            if (displayHeight > canvas.height) {
                displayHeight = canvas.height;
                displayWidth = canvas.height * aspectRatio;
            }
            const offsetX = (canvas.width - displayWidth) / 2;
            const offsetY = (canvas.height - displayHeight) / 2;
            const tempCanvas = document.createElement('canvas');
            tempCanvas.width = width;
            tempCanvas.height = height;
            const tempCtx = tempCanvas.getContext('2d');
            tempCtx.putImageData(imgData, 0, 0);
            ctx.fillStyle = 'white';
            ctx.fillRect(0, 0, canvas.width, canvas.height);
            ctx.drawImage(tempCanvas, offsetX, offsetY, displayWidth, displayHeight);
            ctx.fillStyle = '#333';
            ctx.font = '11px system-ui';
            ctx.textAlign = 'left';
            ctx.fillText(`Min: ${minVal.toFixed(2)}`, 10, canvas.height - 5);
            ctx.textAlign = 'right';
            ctx.fillText(`Max: ${imageData.max_val?.toFixed(2) || 'N/A'}`, canvas.width - 10, canvas.height - 5);
            return;
        }
        
        if (!rawData) {
            throw new Error('No raw data available');
        }

        // Create image from raw data with 99.5 percentile scaling
        const imgData = ctx.createImageData(width, height);
        const data = imgData.data;

        for (let i = 0; i < rawData.length; i++) {
            const val = rawData[i];
            // Clamp to [minVal, p995] and normalize to [0, 1]
            const normalized = Math.max(0, Math.min(1, (val - minVal) / (p995 - minVal)));
            
            // Apply Viridis colormap
            const color = getViridisColor(normalized);
            
            const pixelIdx = i * 4;
            data[pixelIdx] = color[0];     // R
            data[pixelIdx + 1] = color[1]; // G
            data[pixelIdx + 2] = color[2]; // B
            data[pixelIdx + 3] = 255;      // A
        }

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

        // Add colorbar info with 99.5 percentile
        ctx.fillStyle = '#333';
        ctx.font = '11px system-ui';
        ctx.textAlign = 'left';
        ctx.fillText(`Min: ${minVal.toFixed(2)}`, 10, canvas.height - 5);
        ctx.textAlign = 'right';
        ctx.fillText(`P99.5: ${p995.toFixed(2)}`, canvas.width - 10, canvas.height - 5);
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
 * Viridis colormap (perceptually uniform)
 * Maps [0, 1] to RGB color
 */
function getViridisColor(value) {
    // Viridis colormap lookup table
    const viridis = [
        [0.267004, 0.004874, 0.329415],
        [0.282623, 0.140461, 0.469910],
        [0.253935, 0.265254, 0.529983],
        [0.206756, 0.371758, 0.553806],
        [0.163625, 0.471133, 0.558390],
        [0.127568, 0.566949, 0.550413],
        [0.134692, 0.658636, 0.517649],
        [0.266941, 0.748751, 0.440573],
        [0.477504, 0.821444, 0.318195],
        [0.741388, 0.873449, 0.149561],
        [0.993248, 0.906157, 0.143936],
    ];
    
    const idx = value * (viridis.length - 1);
    const lowerIdx = Math.floor(idx);
    const upperIdx = Math.ceil(idx);
    const t = idx - lowerIdx;
    
    if (lowerIdx === upperIdx) {
        const c = viridis[lowerIdx];
        return [Math.round(c[0] * 255), Math.round(c[1] * 255), Math.round(c[2] * 255)];
    }
    
    const c1 = viridis[lowerIdx];
    const c2 = viridis[upperIdx];
    return [
        Math.round((c1[0] * (1 - t) + c2[0] * t) * 255),
        Math.round((c1[1] * (1 - t) + c2[1] * t) * 255),
        Math.round((c1[2] * (1 - t) + c2[2] * t) * 255),
    ];
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
 * Draw 3D LUT geometry with Three.js
 * Z-axis: 2θ (scattering angle)
 * X-Y axes: Detector coordinates
 */
function draw3DLUTGeometry() {
    const container = document.getElementById('lut-3d');
    
    if (!container) {
        console.error('[draw3DLUTGeometry] Container not found');
        return;
    }

    if (!appState.explorer) {
        container.innerHTML = '<p style="color: #999; text-align: center; padding: 20px;">WASM not initialized</p>';
        return;
    }

    try {
        const lutStr = appState.explorer.get_lut();
        const lut = JSON.parse(lutStr);

        if (lut.status !== 'success') {
            container.innerHTML = '<p style="color: #999; text-align: center; padding: 20px;">LUT Error</p>';
            return;
        }

        // Clear previous scene
        container.innerHTML = '';

        // Set up Three.js scene
        const scene = new THREE.Scene();
        scene.background = new THREE.Color(0xffffff);

        // Camera and renderer
        const width = container.clientWidth || 800;
        const height = container.clientHeight || 400;
        const camera = new THREE.PerspectiveCamera(75, width / height, 0.1, 10000);
        camera.position.set(500, 500, 500);
        camera.lookAt(0, 0, 0);

        const renderer = new THREE.WebGLRenderer({ antialias: true });
        renderer.setSize(width, height);
        renderer.setPixelRatio(window.devicePixelRatio || 1);
        container.appendChild(renderer.domElement);

        // Add lighting
        const light1 = new THREE.PointLight(0xffffff, 1, 10000);
        light1.position.set(500, 500, 500);
        scene.add(light1);

        const light2 = new THREE.AmbientLight(0xffffff, 0.5);
        scene.add(light2);

        // Create points from LUT data
        const points = [];
        let minX = Infinity, maxX = -Infinity;
        let minY = Infinity, maxY = -Infinity;
        let minZ = Infinity, maxZ = -Infinity;

        lut.lut_samples.forEach(sample => {
            // Use pixel coordinates as X, Y and 2θ as Z
            points.push([
                sample.pixel_x,
                sample.pixel_y,
                sample.two_theta_deg * 10 // Scale for visibility
            ]);
            minX = Math.min(minX, sample.pixel_x);
            maxX = Math.max(maxX, sample.pixel_x);
            minY = Math.min(minY, sample.pixel_y);
            maxY = Math.max(maxY, sample.pixel_y);
            minZ = Math.min(minZ, sample.two_theta_deg * 10);
            maxZ = Math.max(maxZ, sample.two_theta_deg * 10);
        });

        // Create geometry for points
        const geometry = new THREE.BufferGeometry();
        const positions = new Float32Array(points.length * 3);
        const colors = new Float32Array(points.length * 3);

        points.forEach((point, i) => {
            positions[i * 3] = point[0];
            positions[i * 3 + 1] = point[1];
            positions[i * 3 + 2] = point[2];

            // Color by 2θ value (from 0=blue to 1=red)
            const zNorm = (point[2] - minZ) / (maxZ - minZ || 1);
            const color = getViridisColor(zNorm);
            colors[i * 3] = color[0] / 255;
            colors[i * 3 + 1] = color[1] / 255;
            colors[i * 3 + 2] = color[2] / 255;
        });

        geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
        geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));

        const material = new THREE.PointsMaterial({
            size: 8,
            vertexColors: true,
            sizeAttenuation: true
        });

        const pointsMesh = new THREE.Points(geometry, material);
        scene.add(pointsMesh);

        // Add axes helper
        const axesHelper = new THREE.AxesHelper(100);
        scene.add(axesHelper);

        // Add grid
        const gridHelper = new THREE.GridHelper(1000, 10, 0xcccccc, 0xeeeeee);
        gridHelper.position.z = minZ - 10;
        scene.add(gridHelper);

        // Add axis labels
        const canvas = renderer.domElement.parentElement;
        const info = document.createElement('div');
        info.style.cssText = 'position: absolute; top: 10px; left: 10px; color: #333; font-size: 11px; background: rgba(255,255,255,0.8); padding: 10px; border-radius: 4px;';
        info.innerHTML = `
            <div><strong>LUT 3D Geometry</strong></div>
            <div>X: Detector X (0–${Math.round(maxX)} px)</div>
            <div>Y: Detector Y (0–${Math.round(maxY)} px)</div>
            <div>Z: 2θ (${(minZ/10).toFixed(1)}–${(maxZ/10).toFixed(1)}°)</div>
            <div>Distance: ${lut.distance_m.toFixed(3)} m</div>
            <div style="margin-top: 5px; font-size: 10px; color: #666;">Drag to rotate, scroll to zoom</div>
        `;
        container.appendChild(info);

        // Add mouse controls
        let isDragging = false;
        let previousMousePosition = { x: 0, y: 0 };
        const rotation = { x: 0, y: 0 };

        renderer.domElement.addEventListener('mousedown', (e) => {
            isDragging = true;
            previousMousePosition = { x: e.clientX, y: e.clientY };
        });

        renderer.domElement.addEventListener('mousemove', (e) => {
            if (isDragging) {
                const deltaX = e.clientX - previousMousePosition.x;
                const deltaY = e.clientY - previousMousePosition.y;
                rotation.y += deltaX * 0.005;
                rotation.x += deltaY * 0.005;
                previousMousePosition = { x: e.clientX, y: e.clientY };
                
                pointsMesh.rotation.y = rotation.y;
                pointsMesh.rotation.x = rotation.x;
            }
        });

        renderer.domElement.addEventListener('mouseup', () => {
            isDragging = false;
        });

        renderer.domElement.addEventListener('wheel', (e) => {
            e.preventDefault();
            camera.position.multiplyScalar(1 + e.deltaY * 0.001);
        });

        // Render loop
        function animate() {
            requestAnimationFrame(animate);
            renderer.render(scene, camera);
        }

        animate();
    } catch (error) {
        console.error('[draw3DLUTGeometry] Error:', error);
        container.innerHTML = `<p style="color: #c33; text-align: center; padding: 20px;">Error: ${error.message}</p>`;
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

// Handle window resize for responsive canvas
window.addEventListener('resize', () => {
    const canvas = document.getElementById('canvas');
    const graph = document.getElementById('graph');
    
    if (appState.lastResult) {
        // Redraw with new dimensions
        setTimeout(() => {
            draw2DImage();
            draw1DCurve(appState.lastResult);
            draw3DLUTGeometry();
        }, 100);
    }
});
