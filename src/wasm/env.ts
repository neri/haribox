/**
 * env.ts
 *
 * WASM environment module providing JavaScript host functions
 * Imported by wasm-bindgen generated code (rust_task.js) as the 'env' module
 * rustTask.worker.ts must call initHost() before loading the WASM module
 */

import { readTaskFile, writeTaskFile } from '../fs/taskFileSystem';
import type { TaskFileSystem } from '../fs/taskFileSystem';
import type { WorkerCommand } from '../protocol';

/**
 * State shared between rustTask.worker.ts and the host functions below
 */
export type HostState = {
    fileSystem: TaskFileSystem;
    windowIdMap: Map<number, string>; // Maps numeric windowId to UUID
    nextWindowId: number; // Counter for generating numeric windowIds
    initTime: number; // Performance counter at worker initialization
};

let host: HostState | null = null;

/**
 * Register the state used by the host functions. Pass null to release it.
 */
export const initHost = (state: HostState | null): void => {
    host = state;
};

const getHost = (): HostState => {
    if (!host) {
        throw new Error('WASM environment not initialized. This should only be called from Worker context.');
    }
    return host;
};

/**
 * Send message to main thread
 * Buffers listed in transfer are moved instead of copied and can no longer be used here
 */
const post = (message: WorkerCommand, transfer: Transferable[] = []): void => {
    self.postMessage(message, { transfer });
};

/**
 * js_open_window(width, height, title) -> u32
 * Create a new window and return a numeric handle
 * (設計書 14.2: Rust opens window via this interface)
 *
 * Worker generates a UUID windowId and returns a numeric ID to Rust.
 * The numeric ID is mapped to the UUID internally for other window operations.
 */
export function js_open_window(width: number, height: number, title: string): number {
    const state = getHost();

    // Generate UUID for main thread
    const windowUuid = crypto.randomUUID();

    // Generate numeric ID for Rust
    const numericWindowId = ++state.nextWindowId;

    // Store mapping
    state.windowIdMap.set(numericWindowId, windowUuid);

    // Send to main thread with UUID
    post({
        type: 'openWindow',
        windowId: windowUuid,
        width,
        height,
        title,
    });

    // Return numeric ID to Rust
    return numericWindowId;
}

/**
 * js_move_window(window_id, x, y)
 * Move window to specified coordinates
 */
export function js_move_window(windowId: number, x: number, y: number): void {
    const windowUuid = getHost().windowIdMap.get(windowId);
    if (!windowUuid) {
        console.warn(`[worker] js_move_window: window ID ${windowId} not found`);
        return;
    }
    post({
        type: 'moveWindow',
        windowId: windowUuid,
        x,
        y,
    });
}

/**
 * js_activate_window(window_id)
 * Bring window to front
 */
export function js_activate_window(windowId: number): void {
    const windowUuid = getHost().windowIdMap.get(windowId);
    if (!windowUuid) {
        console.warn(`[worker] js_activate_window: window ID ${windowId} not found`);
        return;
    }
    post({
        type: 'activateWindow',
        windowId: windowUuid,
    });
}

/**
 * js_close_window(window_id)
 * Close specified window
 */
export function js_close_window(windowId: number): void {
    const state = getHost();
    const windowUuid = state.windowIdMap.get(windowId);
    if (!windowUuid) {
        console.warn(`[worker] js_close_window: window ID ${windowId} not found`);
        return;
    }
    post({
        type: 'closeWindow',
        windowId: windowUuid,
    });
    // Remove from mapping
    state.windowIdMap.delete(windowId);
}

/**
 * js_draw_image(window_id, x, y, width, height, pixels)
 * Draw RGBA image data to specified rectangular region on canvas window
 * pixels is a view into WASM memory that is only valid during this call
 * (設計書 14.2: Partial canvas drawing)
 */
export function js_draw_image(windowId: number, x: number, y: number, width: number, height: number, pixels: Uint8Array): void {
    const state = getHost();
    const windowUuid = state.windowIdMap.get(windowId);
    if (!windowUuid) {
        console.warn(`[worker] js_draw_image: window ID ${windowId} not found`);
        return;
    }
    // Copy the data out of WASM memory, then hand the copy over to the main thread without copying it again
    const copy = pixels.slice().buffer;
    post(
        {
            type: 'drawImage',
            windowId: windowUuid,
            x,
            y,
            width,
            height,
            pixels: copy,
        },
        [copy],
    );
}

/**
 * js_print(text)
 * Print text to terminal without newline
 * (設計書 14.2: Rust prints via this interface)
 */
export function js_print(text: string): void {
    post({ type: 'print', text });
}

/**
 * js_read_file(filename) -> Option<Vec<u8>>
 * Get file content, or undefined if not found
 * The content is copied into WASM memory by the wasm-bindgen glue code
 */
export function js_read_file(filename: string): Uint8Array | undefined {
    return readTaskFile(getHost().fileSystem, filename);
}

/**
 * js_write_file(filename, data, mode) -> i32
 * Write file content
 * data is a view into WASM memory that is only valid during this call
 * mode: 0=update, 1=create, 2=upsert
 * Returns 0 on success, negative on error
 */
export function js_write_file(filename: string, data: Uint8Array, mode: number): number {
    const content = data.slice(); // Copy the data out of WASM memory
    const name = writeTaskFile(getHost().fileSystem, filename, content, mode);
    if (name === null) {
        return -1;
    }

    // Notify main thread of file write (for persistence)
    post({
        type: 'fileWritten',
        filename: name,
        data: content.buffer,
    });

    return 0; // Success
}

/**
 * js_get_tick() -> f64
 * Get elapsed time in milliseconds since Worker initialization
 * Returns the difference between current performance.now() and initTime
 */
export function js_get_tick(): number {
    return performance.now() - getHost().initTime;
}

/**
 * js_play_sound(frequency)
 * Play sound with specified frequency (Hz)
 * frequency > 0: play sound at that frequency
 * frequency = 0: stop sound
 * (設計書 Oscillator 音声再生機能)
 */
export function js_play_sound(frequency: number): void {
    post({
        type: 'playSound',
        frequency,
        timestamp: performance.now(),
    });
}
