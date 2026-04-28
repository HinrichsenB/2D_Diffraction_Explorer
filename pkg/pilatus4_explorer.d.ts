/* tslint:disable */
/* eslint-disable */

/**
 * Main WASM interface for data processing
 */
export class DataExplorer {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get detector info
     */
    detector_info(): string;
    /**
     * Load bright field correction from base64-encoded .npy
     */
    load_bright_field(npy_base64: string): string;
    /**
     * Load sample image from base64-encoded .tiff
     */
    load_image(tiff_base64: string): string;
    /**
     * Load pixel mask from base64-encoded .edf
     */
    load_mask(edf_base64: string): string;
    /**
     * Load PONI calibration from text content
     */
    load_poni(poni_content: string): string;
    /**
     * Create a new data explorer instance
     */
    constructor();
    /**
     * Process loaded data: apply corrections and integration
     */
    process(tth_min: number, tth_max: number, n_bins: number): string;
    /**
     * Get current data status
     */
    status(): string;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_dataexplorer_free: (a: number, b: number) => void;
    readonly dataexplorer_detector_info: (a: number) => [number, number];
    readonly dataexplorer_load_bright_field: (a: number, b: number, c: number) => [number, number, number, number];
    readonly dataexplorer_load_poni: (a: number, b: number, c: number) => [number, number, number, number];
    readonly dataexplorer_new: () => number;
    readonly dataexplorer_process: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly dataexplorer_status: (a: number) => [number, number];
    readonly dataexplorer_load_image: (a: number, b: number, c: number) => [number, number, number, number];
    readonly dataexplorer_load_mask: (a: number, b: number, c: number) => [number, number, number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
