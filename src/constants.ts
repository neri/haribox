export const APP_NAME = 'HariboteBox';
export const FILE_IMPORT_SIZE_LIMIT_BYTES = 1.5 * 1024 * 1024;
export const FILE_STORAGE_KEY = 'haribote.fs.v1';
export const FILE_STORAGE_WARNING_BYTES = 1024 * 1024;
export const INITIAL_TERMINAL_WINDOW_HEIGHT = 320;
export const INITIAL_TERMINAL_WINDOW_WIDTH = 520;
export const INITIAL_TERMINAL_X = 40;
export const INITIAL_TERMINAL_Y = 40;
export const TASKBAR_HEIGHT_PX = 48;
export const TERMINAL_WINDOW_OFFSET_STEP = 24;
export const TERMINAL_MAX_LINES = 4000;
export const TEXT_DECODER = new TextDecoder();
export const TEXT_ENCODER = new TextEncoder();
export const TITLE_BAR_HEIGHT = 32;
export const Z_INDEX_REFRESH_THRESHOLD = 10_000;

// App IDs (UUIDv4 format)
export const APP_IDS = {
  TERMINAL: 'f9c271dc-49ee-4295-811a-dc2bf7bd27a7',
  CANVAS: '8faf5f5e-66ee-4580-8a8f-fe64ed78ea23',
  FILE_MANAGER: 'c1a107bb-20ec-4d3d-ad94-944d5bce863d',
  ABOUT: '88a37a04-c8e5-42e6-9f13-2f7fd31f62c3',
  TEXT_VIEWER: 'e97b8139-cebf-40cd-8805-a0b87192c50f',
  ONBOARDING: 'd1b3e8f4-7a2e-4c9d-b5f1-9a8c6d2e4f3b',
  SYSTEM_MODAL: 'a4e9f2c1-5d7b-4e2a-9c3f-8b1d6a5e2f9c',
} as const;

// Injected by Vite (see vite.config.ts)
declare const __APP_VERSION__: string;
declare const __GIT_HASH__: string;
export const APP_VERSION = __APP_VERSION__;
export const GIT_HASH = __GIT_HASH__;
