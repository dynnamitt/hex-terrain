/* tslint:disable */
/* eslint-disable */

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly main: (a: number, b: number) => number;
    readonly wasm_bindgen__closure__destroy__h23caa69c660708ab: (a: number, b: number) => void;
    readonly wasm_bindgen__closure__destroy__h37624f14b2cf6ddf: (a: number, b: number) => void;
    readonly wasm_bindgen__closure__destroy__h0eaf5a3ee197dbf1: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h89aec82e2cab33ae: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2e680c794b707af0: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__ha68bd66e72012e21: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h10c999c6abd910bd: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2488dbc689765e71: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
