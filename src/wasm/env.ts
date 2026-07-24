/**
 * env.ts
 * 
 * WASM environment module providing JavaScript host functions
 * Imported by wasm-bindgen generated code (rust_task.js)
 * At runtime in Worker context, these delegate to globalThis.wasmEnv
 */

// Get the WASM environment object set by rustTask.worker.ts
const wasmEnv = (globalThis as any).wasmEnv;

const getEnv = () => {
    if (!wasmEnv) {
        throw new Error('WASM environment not initialized. This should only be called from Worker context.');
    }
    return wasmEnv;
};

/* eslint-disable @typescript-eslint/no-unused-vars */

export function js_open_window(width: number, height: number, titlePtr: number, titleLen: number): number {
    return getEnv().js_open_window(width, height, titlePtr, titleLen);
}

export function js_move_window(windowId: number, x: number, y: number): void {
    return getEnv().js_move_window(windowId, x, y);
}

export function js_activate_window(windowId: number): void {
    return getEnv().js_activate_window(windowId);
}

export function js_close_window(windowId: number): void {
    return getEnv().js_close_window(windowId);
}

export function js_draw_image(windowId: number, x: number, y: number, width: number, height: number, ptr: number, len: number): void {
    return getEnv().js_draw_image(windowId, x, y, width, height, ptr, len);
}

export function js_print(textPtr: number, textLen: number): void {
    return getEnv().js_print(textPtr, textLen);
}

export function js_read_file_size(filenamePtr: number, filenameLen: number): number {
    return getEnv().js_read_file_size(filenamePtr, filenameLen);
}

export function js_read_file_into(bufPtr: number, bufLen: number): number {
    return getEnv().js_read_file_into(bufPtr, bufLen);
}

export function js_write_file(filenamePtr: number, filenameLen: number, dataPtr: number, dataLen: number, mode: number): number {
    return getEnv().js_write_file(filenamePtr, filenameLen, dataPtr, dataLen, mode);
}

export function js_get_keyboard_event(windowId: number): number {
    return getEnv().js_get_keyboard_event(windowId);
}

export function js_get_tick(): number {
    return getEnv().js_get_tick();
}

export function js_schedule_event(delay_ms: number, event_code: number): void {
    return getEnv().js_schedule_event(delay_ms, event_code);
}

export function js_play_sound(frequency: number): void {
    return getEnv().js_play_sound(frequency);
}

/* eslint-enable @typescript-eslint/no-unused-vars */
