/**
 * rustTask.worker.ts
 * 
 * Worker thread for running Rust(WASM) tasks
 * Manages WASM initialization, environment setup, and execution
 * 
 * Design reference: Section 14 (Rust/Wasm インターフェース設計) in design.md
 */

/**
 * Message type received from main thread to start a command execution
 */
type StartMessage = {
    type: 'startWithCommand';
    terminalWindowId: string;
    fileName: string;
    commandLine: string;
    titleBarHeight: number;
    fileSystemSnapshot: Array<{ name: string; content: ArrayBuffer }>;
    environmentVariables: Record<string, string>;
};

/**
 * Keyboard event message sent from main thread
 */
type KeyboardEventMessage = {
    type: 'keyboardEvent';
    windowId: string;
    eventType: 'keydown' | 'keyup';
    key: string;
    code: string;
    keyCode: number;
    ctrlKey: boolean;
    shiftKey: boolean;
    altKey: boolean;
    metaKey: boolean;
    isAutoRepeat: boolean;
    modifierBitmap: number;
};

/**
 * Window close message sent from main thread
 */
type WindowCloseMessage = {
    type: 'windowClose';
    windowId: string;
};

/**
 * Global context for WASM environment functions
 * These functions are called by Rust/WASM code via js_import_module
 */
type WasmEnv = {
    js_open_window: (width: number, height: number, titlePtr: number, titleLen: number) => number;
    js_move_window: (windowId: number, x: number, y: number) => void;
    js_activate_window: (windowId: number) => void;
    js_close_window: (windowId: number) => void;
    js_draw_image: (windowId: number, x: number, y: number, width: number, height: number, ptr: number, len: number) => void;
    js_print: (textPtr: number, textLen: number) => void;
    js_read_file_size: (filenamePtr: number, filenameLen: number) => number;
    js_read_file_into: (bufPtr: number, bufLen: number) => number;
    js_write_file: (filenamePtr: number, filenameLen: number, dataPtr: number, dataLen: number, mode: number) => number;
    js_get_keyboard_event: (windowId: number) => number;
    js_get_tick: () => number;
    js_schedule_event: (delay_ms: number, event_code: number) => void;
    js_play_sound: (frequency: number) => void;
};

/**
 * File system entry
 */
type FileSystemEntry = {
    name: string;
    content: Uint8Array;
};

/**
 * Worker state
 */
let workerState: {
    terminalWindowId: string;
    fileName: string;
    commandLine: string;
    titleBarHeight: number;
    fileSystem: Map<string, FileSystemEntry>;
    env: Record<string, string>;
    wasmMemory: WebAssembly.Memory;
    lastReadFile: { content: Uint8Array; offset: number };
    windowIdMap: Map<number, string>; // Maps numeric windowId to UUID
    nextWindowId: number; // Counter for generating numeric windowIds
    eventQueue: number[]; // Single event queue for all windows in this Worker instance
    initTime: number; // Performance counter at worker initialization
    workerId: string; // UUID for this worker instance (for audio playSound)
} | null = null;

/**
 * Send message to main thread
 */
const post = (message: unknown): void => {
    self.postMessage(message);
};

/**
 * Send text output to specified terminal window
 * (設計書 14.3: println with newline)
 */
const println = (windowId: string, text: string): void => {
    post({ type: 'println', windowId, text });
};

/**
 * Send text output without newline to specified terminal window
 * (設計書 14.3: print without newline)
 */
const print = (windowId: string, text: string): void => {
    post({ type: 'print', windowId, text });
};

/**
 * Generate a UUID v4 for windowId
 */
const generateUUID = (): string => {
    return crypto.randomUUID();
};

/**
 * Handle keyboard event from main thread
 * Enqueues the keyboard event to the Worker's global event queue
 */
const handleKeyboardEvent = (event: KeyboardEventMessage): void => {
    if (!workerState) {
        console.warn('[worker] Worker state not initialized');
        return;
    }

    // Convert key to numeric event code
    let eventCode: number = -1; // TBD for special keys

    // For keydown events, check if it's a printable character
    if (event.eventType === 'keydown' && event.key.length === 1) {
        const charCode = event.key.charCodeAt(0);
        // Only enqueue if it's a printable ASCII character (0x20-0x7E)
        if (charCode >= 0x20 && charCode <= 0x7E) {
            eventCode = charCode;
        }
    } else if (event.eventType === 'keydown') {
        // Handle special keys (Enter, Arrow keys, Escape, etc.)
        switch (event.code) {
            case 'Backspace': eventCode = 0x08; break; // Backspace
            case 'Enter': eventCode = 0x0a; break;// Carriage Return
            case 'Escape': eventCode = 0x1b; break; // Escape
            case 'PageUp': eventCode = 0x80; break; // Custom code for Page Up
            case 'PageDown': eventCode = 0x81; break; // Custom code for Page Down
            case 'End': eventCode = 0x82; break; // Custom code for End
            case 'Home': eventCode = 0x83; break; // Custom code for Home
            case 'ArrowLeft': eventCode = 0x84; break; // Custom code for Arrow Left
            case 'ArrowRight': eventCode = 0x85; break; // Custom code for Arrow Right
            case 'ArrowUp': eventCode = 0x86; break; // Custom code for Arrow Up
            case 'ArrowDown': eventCode = 0x87; break; // Custom code for Arrow Down
            case 'Insert': eventCode = 0x88; break; // Custom code for Insert
            case 'Delete': eventCode = 0x89; break; // Custom code for Delete
        }
    }

    // Only enqueue if we have a valid event code
    if (eventCode >= 0) {
        // Encode modifier bitmap into upper byte (bit 8-15)
        // USB HID modifier format:
        // bit0: Left Ctrl, bit1: Left Shift, bit2: Left Alt, bit3: Left GUI
        // bit4: Right Ctrl, bit5: Right Shift, bit6: Right Alt, bit7: Right GUI
        // Maps to:
        // bit8: Left Shift, bit9: Left Ctrl, bit10: Left Alt, bit11: -
        // bit12: Right Shift, bit13: Right Ctrl, bit14: Right Alt, bit15: -
        let eventWithModifier = eventCode;
        const bitmap = event.modifierBitmap;

        // Convert USB HID format to legacy haribote format
        let modifierCode = 0;
        if (bitmap & 0x01) modifierCode |= 0x02; // Left Ctrl -> bit 9
        if (bitmap & 0x02) modifierCode |= 0x01; // Left Shift -> bit 8
        if (bitmap & 0x04) modifierCode |= 0x04; // Left Alt -> bit 10
        if (bitmap & 0x08) modifierCode |= 0x08; // Left GUI -> bit 11
        if (bitmap & 0x10) modifierCode |= 0x20; // Right Ctrl -> bit 13
        if (bitmap & 0x20) modifierCode |= 0x10; // Right Shift -> bit 12
        if (bitmap & 0x40) modifierCode |= 0x40; // Right Alt -> bit 14
        if (bitmap & 0x80) modifierCode |= 0x80; // Right GUI -> bit 15

        eventWithModifier = eventCode | (modifierCode << 8);
        workerState.eventQueue.push(eventWithModifier);
        // console.log(`[worker] Enqueued event code 0x${eventWithModifier.toString(16)} (modifier=0x${event.modifierBitmap.toString(16)})`);
    } else {
        // console.log(`[worker] Skipped event: ${event.eventType} - ${modifierStr}${event.key} (code: ${event.code})`);
    }
};

/**
 * Timer ID for deferred worker shutdown check
 * Used to track if we've already scheduled a shutdown check
 */
let workerShutdownCheckTimerId: number | null = null;

/**
 * Handle window close message from main thread
 * Cleans up windowIdMap and triggers worker shutdown if no windows remain
 */
const handleWindowClose = (message: WindowCloseMessage): void => {
    if (!workerState) {
        console.warn('[worker] Worker state not initialized');
        return;
    }

    console.log(`[worker] Window Closed: ${message.windowId}`);

    // Find and remove the numeric window ID from windowIdMap
    let numericWindowId: number | null = null;
    for (const [numId, uuidId] of workerState.windowIdMap.entries()) {
        if (uuidId === message.windowId) {
            numericWindowId = numId;
            break;
        }
    }

    if (numericWindowId !== null) {
        workerState.windowIdMap.delete(numericWindowId);
        console.log(`[worker] Cleaned up window ID mapping: numeric=${numericWindowId}, uuid=${message.windowId}`);
    } else {
        console.warn(`[worker] Window ID ${message.windowId} not found in windowIdMap`);
    }

    // Check if all windows have been closed
    if (workerState.windowIdMap.size === 0) {
        console.log('[worker] All windows closed. Scheduling worker shutdown check...');

        // Clear any existing timer
        if (workerShutdownCheckTimerId !== null) {
            clearTimeout(workerShutdownCheckTimerId);
        }

        // Set 100-millisecond timer to check again
        workerShutdownCheckTimerId = setTimeout(() => {
            workerShutdownCheckTimerId = null;

            // Check again if windowIdMap is still empty
            if (workerState && workerState.windowIdMap.size === 0) {
                console.log('[worker] No windows remain after 100ms check. Terminating worker.');
                post({ type: 'done' });
            } else {
                console.log('[worker] Windows reopened during shutdown check. Worker continues.');
            }
        }, 100);
    }
};

/**
 * Read UTF-8 string from WASM memory
 */
const readStringFromMemory = (ptr: number, len: number): string => {
    if (!workerState?.wasmMemory) {
        throw new Error('WASM memory not initialized');
    }
    const buffer = new Uint8Array(workerState.wasmMemory.buffer, ptr, len);
    const decoder = new TextDecoder('utf-8');
    return decoder.decode(buffer);
};

/**
 * Create WASM environment functions
 * These are called from Rust/WASM code
 */
const createWasmEnv = (): WasmEnv => {
    return {
        /**
         * js_open_window(width, height, title_ptr, title_len) -> u32
         * Create a new window and return a numeric handle
         * (設計書 14.2: Rust opens window via this interface)
         * 
         * Worker generates a UUID windowId and returns a numeric ID to Rust.
         * The numeric ID is mapped to the UUID internally for other window operations.
         */
        js_open_window: (width: number, height: number, titlePtr: number, titleLen: number): number => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            const title = readStringFromMemory(titlePtr, titleLen);

            // Generate UUID for main thread
            const windowUuid = generateUUID();

            // Generate numeric ID for Rust
            const numericWindowId = ++workerState.nextWindowId;

            // Store mapping
            workerState.windowIdMap.set(numericWindowId, windowUuid);

            // Send to main thread with UUID
            post({
                type: 'openWindow',
                windowType: 'canvas',
                windowId: windowUuid,
                width,
                height,
                title,
            });

            // Return numeric ID to Rust
            return numericWindowId;
        },

        /**
         * js_move_window(window_id, x, y)
         * Move window to specified coordinates
         */
        js_move_window: (_windowId: number, x: number, y: number): void => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            const windowUuid = workerState.windowIdMap.get(_windowId);
            if (!windowUuid) {
                console.warn(`[worker] js_move_window: window ID ${_windowId} not found`);
                return;
            }
            post({
                type: 'moveWindow',
                windowId: windowUuid,
                x,
                y,
            });
        },

        /**
         * js_activate_window(window_id)
         * Bring window to front
         */
        js_activate_window: (_windowId: number): void => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            const windowUuid = workerState.windowIdMap.get(_windowId);
            if (!windowUuid) {
                console.warn(`[worker] js_activate_window: window ID ${_windowId} not found`);
                return;
            }
            post({
                type: 'activateWindow',
                windowId: windowUuid,
            });
        },

        /**
         * js_close_window(window_id)
         * Close specified window
         */
        js_close_window: (_windowId: number): void => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            const windowUuid = workerState.windowIdMap.get(_windowId);
            if (!windowUuid) {
                console.warn(`[worker] js_close_window: window ID ${_windowId} not found`);
                return;
            }
            post({
                type: 'closeWindow',
                windowId: windowUuid,
            });
            // Remove from mapping
            workerState.windowIdMap.delete(_windowId);
        },

        /**
         * js_draw_image(window_id, x, y, width, height, ptr, len)
         * Draw RGBA image data to specified rectangular region on canvas window
         * (設計書 14.2: Partial canvas drawing)
         */
        js_draw_image: (_windowId: number, x: number, y: number, width: number, height: number, ptr: number, len: number): void => {
            if (!workerState?.wasmMemory) {
                throw new Error('WASM memory not initialized');
            }
            const windowUuid = workerState.windowIdMap.get(_windowId);
            if (!windowUuid) {
                console.warn(`[worker] js_draw_image: window ID ${_windowId} not found`);
                return;
            }
            const imageData = new Uint8Array(workerState.wasmMemory.buffer, ptr, len);
            post({
                type: 'drawImage',
                windowId: windowUuid,
                x,
                y,
                width,
                height,
                pixels: imageData.slice(), // Copy the data (ArrayBuffer-like)
            });
        },

        /**
         * js_print(text_ptr, text_len)
         * Print text to terminal without newline
         * (設計書 14.2: Rust prints via this interface)
         */
        js_print: (textPtr: number, textLen: number): void => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            const text = readStringFromMemory(textPtr, textLen);
            print(workerState.terminalWindowId, text);
        },

        /**
         * js_play_sound(frequency)
         * Play sound with specified frequency (Hz)
         * frequency > 0: play sound at that frequency
         * frequency = 0: stop sound
         * (設計書 Oscillator 音声再生機能)
         */
        js_play_sound: (frequency: number): void => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            post({
                type: 'playSound',
                workerId: workerState.workerId,
                frequency,
                timestamp: performance.now(),
            });
        },

        /**
         * js_read_file_size(filename_ptr, filename_len) -> i32
         * Get file size in bytes, or negative if not found
         */
        js_read_file_size: (filenamePtr: number, filenameLen: number): number => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            const filename = readStringFromMemory(filenamePtr, filenameLen);
            const canonicalKey = filename.toUpperCase();
            const entry = workerState.fileSystem.get(canonicalKey);

            if (!entry) {
                return -1; // File not found
            }

            workerState.lastReadFile = {
                content: entry.content,
                offset: 0,
            };

            return entry.content.length;
        },

        /**
         * js_read_file_into(buf_ptr, buf_len) -> i32
         * Read file content into buffer, returns bytes read
         */
        js_read_file_into: (bufPtr: number, bufLen: number): number => {
            if (!workerState?.wasmMemory || !workerState.lastReadFile) {
                throw new Error('File read not initialized');
            }

            const buffer = new Uint8Array(workerState.wasmMemory.buffer, bufPtr, bufLen);
            const bytesToRead = Math.min(bufLen, workerState.lastReadFile.content.length - workerState.lastReadFile.offset);

            buffer.set(
                workerState.lastReadFile.content.subarray(
                    workerState.lastReadFile.offset,
                    workerState.lastReadFile.offset + bytesToRead,
                ),
            );

            workerState.lastReadFile.offset += bytesToRead;
            return bytesToRead;
        },

        /**
         * js_write_file(filename_ptr, filename_len, data_ptr, data_len, mode) -> i32
         * Write file content
         * mode: 0=update, 1=create, 2=upsert
         * Returns 0 on success, negative on error
         */
        js_write_file: (filenamePtr: number, filenameLen: number, dataPtr: number, dataLen: number, mode: number): number => {
            if (!workerState?.wasmMemory) {
                throw new Error('Worker state not initialized');
            }

            const filename = readStringFromMemory(filenamePtr, filenameLen);
            const canonicalKey = filename.toUpperCase();
            const data = new Uint8Array(workerState.wasmMemory.buffer, dataPtr, dataLen).slice();

            const exists = workerState.fileSystem.has(canonicalKey);

            if (mode === 0 && !exists) return -1; // update: file must exist
            if (mode === 1 && exists) return -1; // create: file must not exist
            // mode === 2: upsert - always succeeds

            workerState.fileSystem.set(canonicalKey, {
                name: filename,
                content: data,
            });

            // Notify main thread of file write (for persistence)
            post({
                type: 'fileWritten',
                filename,
                data: data.buffer,
                mode,
            });

            return 0; // Success
        },

        /**
         * js_get_keyboard_event(window_id) -> i32
         * Get the next keyboard event from the Worker's global event queue
         * Returns event code (-1 if queue is empty)
         * Event codes: ASCII codes for character keys, -1 for empty queue
         * (設計書 14.2: Event queue interface)
         * 
         * Note: window_id parameter is accepted for compatibility but not used.
         * All windows share the same global event queue in this Worker instance.
         */
        js_get_keyboard_event: (_windowId: number): number => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }

            // Dequeue and return the first event from global queue, or -1 if queue is empty
            const event = workerState.eventQueue.shift();
            return event ?? -1;
        },

        /**
         * js_get_tick() -> f64
         * Get elapsed time in milliseconds since Worker initialization
         * Returns the difference between current performance.now() and initTime
         */
        js_get_tick: (): number => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            return performance.now() - workerState.initTime;
        },

        /**
         * js_schedule_event(delay_ms, event_code)
         * Schedule an event to be enqueued after specified delay
         * Uses setTimeout to defer event insertion into the global event queue
         * 
         * Parameters:
         *   delay_ms: Delay in milliseconds before event is enqueued
         *   event_code: Event code to enqueue (typically ASCII code or special code)
         */
        js_schedule_event: (delay_ms: number, event_code: number): void => {
            if (!workerState) {
                throw new Error('Worker state not initialized');
            }
            setTimeout(() => {
                if (workerState) {
                    workerState.eventQueue.push(event_code);
                    // console.log(`[worker] Scheduled event enqueued after ${delay_ms}ms: code ${event_code}`);
                }
            }, delay_ms);
        },
    };
};

/**
 * Main message handler
 */
self.addEventListener('message', async (event: MessageEvent) => {
    const message = event.data;

    // Handle keyboard events from main thread
    if (message.type === 'keyboardEvent') {
        handleKeyboardEvent(message);
        return;
    }

    // Handle window close events from main thread
    if (message.type === 'windowClose') {
        handleWindowClose(message);
        return;
    }

    // Handle file system snapshot updates from main thread
    if (message.type === 'updateFileSystemSnapshot') {
        if (!workerState) {
            return;
        }
        // Update file system with latest snapshot from main
        workerState.fileSystem.clear();
        for (const entry of message.fileSystemSnapshot) {
            workerState.fileSystem.set(entry.name.toUpperCase(), {
                name: entry.name,
                content: new Uint8Array(entry.content),
            });
        }
        return;
    }

    // Handle task start request
    if (message.type !== 'startWithCommand') {
        return;
    }

    const startData: StartMessage = message;

    // console.log('[worker] Received startWithCommand message');

    try {
        // 1. Initialize worker state with placeholder memory
        const fileSystem = new Map<string, FileSystemEntry>();
        for (const entry of startData.fileSystemSnapshot) {
            fileSystem.set(entry.name.toUpperCase(), {
                name: entry.name,
                content: new Uint8Array(entry.content),
            });
        }

        // Create placeholder memory - will be replaced with actual WASM module memory
        let wasmMemory = new WebAssembly.Memory({ initial: 256, maximum: 512 });

        workerState = {
            terminalWindowId: startData.terminalWindowId,
            fileName: startData.fileName,
            commandLine: startData.commandLine,
            titleBarHeight: startData.titleBarHeight,
            fileSystem,
            env: startData.environmentVariables,
            wasmMemory,
            lastReadFile: { content: new Uint8Array(), offset: 0 },
            windowIdMap: new Map(),
            nextWindowId: 0,
            eventQueue: [],
            initTime: performance.now(),
            workerId: crypto.randomUUID(),
        };

        // 2. Create WASM environment object
        const wasmEnv = createWasmEnv();

        // 3. Register environment functions globally for env.ts to use
        // (env.ts imports functions from 'env' module and accesses them via globalThis)
        (globalThis as any).wasmEnv = wasmEnv;

        // 4. Load and initialize WASM module
        // (generated by wasm-bindgen)
        // console.log('[worker] Loading WASM module...');
        let RustTask: any;
        try {
            RustTask = await import('./wasm/rust_task');
            // console.log('[worker] WASM module loaded successfully');
        } catch (loadError) {
            const loadMsg = loadError instanceof Error ? loadError.message : String(loadError);
            console.error('[worker] Failed to load WASM module:', loadError);
            throw new Error(`WASM module load failed: ${loadMsg}`);
        }

        // Initialize WASM runtime and get the actual WASM instance exports
        let wasmExports: any;
        try {
            if (typeof RustTask.default === 'function') {
                // console.log('[worker] Initializing WASM runtime...');
                wasmExports = await RustTask.default();
                // console.log('[worker] WASM runtime initialized successfully');
            } else {
                console.warn('[worker] RustTask.default is not a function');
            }
        } catch (initError) {
            const initMsg = initError instanceof Error ? initError.message : String(initError);
            console.error('[worker] Failed to initialize WASM runtime:', initError);
            throw new Error(`WASM initialization failed: ${initMsg}`);
        }

        // 5. Replace placeholder memory with actual WASM module memory
        // The rust_task.js __wbg_init returns the wasm instance exports which contains the actual memory
        if (wasmExports?.memory instanceof WebAssembly.Memory) {
            wasmMemory = wasmExports.memory;
            if (workerState) {
                workerState.wasmMemory = wasmMemory;
            }
            // console.log('[worker] WASM memory setup complete');
        } else {
            console.warn('[worker] WASM memory not properly initialized');
        }

        // 6. Execute Rust task with parameters
        // (設計書 14.1: run_task is the main entry point)
        try {
            // console.log('[worker] Starting Rust task execution...');
            RustTask.run_task(
                startData.fileName,
                startData.commandLine,
                startData.titleBarHeight,
            );
        } catch (execError) {
            const execMsg = execError instanceof Error ? execError.message : String(execError);
            console.error('[worker] Rust task execution failed:', execError);
            throw new Error(`Task execution failed: ${execMsg}`);
        }

        while (true) {
            const result = RustTask.loop();
            if (result < 0) {
                break;
            }
            await new Promise(resolve => setTimeout(resolve, result));
        }

        // 7. Signal completion
        // println(startData.terminalWindowId, '[worker] Task completed successfully');
        post({ type: 'done' });
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        const stack = error instanceof Error ? error.stack : '';

        // Log to console for visibility in all browsers
        console.error('[worker] Execution error:', error);
        console.error('[worker] Stack trace:', stack);

        // Send error output to terminal
        println(startData.terminalWindowId, `[worker] Error: ${message}`);
        // if (stack) {
        //     println(startData.terminalWindowId, `[worker] Stack: ${stack}`);
        // }

        post({
            type: 'error',
            message,
            stack,
        });
    } finally {
        // Clean up worker state
        workerState = null;
    }
});
