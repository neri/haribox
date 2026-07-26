import './style.css';
import { deflate, inflate } from 'pako';
import Encoding from 'encoding-japanese';
import { INITIAL_FS_ENTRIES } from './generated-initial-fs';
import iconAppWindow from './assets/icons/app-window.svg?raw';
import iconFileText from './assets/icons/file-text.svg?raw';
import iconFile from './assets/icons/file.svg?raw';
import iconPhoto from './assets/icons/photo.svg?raw';
import iconFileMusic from './assets/icons/file-music.svg?raw';
import iconTerminal from './assets/icons/terminal-2.svg?raw';
import iconFolder from './assets/icons/folder.svg?raw';
import iconCategory from './assets/icons/category.svg?raw';
import iconMenu from './assets/icons/box-multiple.svg?raw';
import iconClose from './assets/icons/x.svg?raw';
import iconVolume from './assets/icons/volume.svg?raw';
import iconVolume2 from './assets/icons/volume-2.svg?raw';
import iconVolume3 from './assets/icons/volume-3.svg?raw';
import iconVolume4 from './assets/icons/volume-4.svg?raw';
import blissImage from './assets/bliss.png';

type WindowId = string;
type AppId = string;
type WindowKind = 'canvas' | 'terminal' | 'filemanager' | 'about' | 'textviewer' | 'onboarding';

declare const __APP_VERSION__: string;
declare const __GIT_HASH__: string;

type WindowModel = {
  id: WindowId;
  appId: AppId;
  kind: WindowKind;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
  isActive: boolean;
};

type WindowGroupInfo = {
  appId: AppId;
  kind: WindowKind;
  windowIds: WindowId[];
  isExpanded: boolean;
};

type AppState = {
  windows: WindowModel[];
  nextZIndex: number;
  env: Record<string, string>;
  isMenuOpen: boolean;
  defaultTerminalWindowId: WindowId | null;
  activeWindowId: WindowId | null;
  windowGroups: WindowGroupInfo[];
  audioContext?: AudioContext;
  globalGain?: GainNode;
  oscillators: Map<string, {
    osc: OscillatorNode;
    gain: GainNode;
    startTime: number;
    workerTimestamp: number;
  }>;
  globalVolume: number;
  volumePopupVisible: boolean;
  audioContextState: 'suspended' | 'running' | 'closed';
};

type DragState = {
  id: WindowId;
  pointerId: number;
  startClientX: number;
  startClientY: number;
  originX: number;
  originY: number;
};

type CanvasImage = {
  x: number;
  y: number;
  width: number;
  height: number;
  pixels: ArrayBuffer;
};

type FileEntry = {
  name: string;
  content: Uint8Array;
  isInitialFile: boolean;
};

type StoredFileSystemV1 = {
  version: 1;
  files: Array<{
    name: string;
    contentBase64: string;
  }>;
};

const WriteFileMode = {
  Update: 'update',
  Create: 'create',
  Upsert: 'upsert',
} as const;
type WriteFileMode = (typeof WriteFileMode)[keyof typeof WriteFileMode];

type WorkerCommand =
  | { type: 'openWindow'; windowId: string; width: number; height: number; title: string }
  | { type: 'moveWindow'; windowId: string; x: number; y: number }
  | { type: 'activateWindow'; windowId: string }
  | { type: 'closeWindow'; windowId: string }
  | { type: 'drawImage'; windowId: string; x: number; y: number; width: number; height: number; pixels: ArrayBuffer }
  | { type: 'print'; windowId: string; text: string }
  | { type: 'println'; windowId: string; text: string }
  | { type: 'writeFile'; filename: string; data: ArrayBuffer; mode: WriteFileMode }
  | { type: 'playSound'; workerId: string; frequency: number; timestamp: number }
  | { type: 'error'; message: string }
  | { type: 'done' };

type UpdateFileSystemSnapshotMessage = {
  type: 'updateFileSystemSnapshot';
  fileSystemSnapshot: Array<{ name: string; content: ArrayBuffer }>;
};

type WorkerStartMessage = {
  type: 'startWithCommand';
  seed: number;
  titleBarHeight: number;
  terminalWindowId: string;
  fileName: string;
  commandLine: string;
  fileSystemSnapshot: Array<{ name: string; content: ArrayBuffer }>;
  environmentVariables: Record<string, string>;
};

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

type WindowCloseMessage = {
  type: 'windowClose';
  windowId: string;
};

const APP_NAME = 'HariboteBox';
const FILE_IMPORT_SIZE_LIMIT_BYTES = 1.5 * 1024 * 1024;
const FILE_STORAGE_KEY = 'haribote.fs.v1';
const FILE_STORAGE_WARNING_BYTES = 1024 * 1024;
const INITIAL_TERMINAL_WINDOW_HEIGHT = 320;
const INITIAL_TERMINAL_WINDOW_WIDTH = 520;
const INITIAL_TERMINAL_X = 40;
const INITIAL_TERMINAL_Y = 40;
const TASKBAR_HEIGHT_PX = 48;
const TERMINAL_WINDOW_OFFSET_STEP = 24;
const TERMINAL_MAX_LINES = 4000;
const TEXT_DECODER = new TextDecoder();
const TEXT_ENCODER = new TextEncoder();
const TITLE_BAR_HEIGHT = 32;
const Z_INDEX_REFRESH_THRESHOLD = 10_000;

// App IDs (UUIDv4 format)
const APP_IDS = {
  TERMINAL: 'f9c271dc-49ee-4295-811a-dc2bf7bd27a7',
  CANVAS: '8faf5f5e-66ee-4580-8a8f-fe64ed78ea23',
  FILE_MANAGER: 'c1a107bb-20ec-4d3d-ad94-944d5bce863d',
  ABOUT: '88a37a04-c8e5-42e6-9f13-2f7fd31f62c3',
  TEXT_VIEWER: 'e97b8139-cebf-40cd-8805-a0b87192c50f',
  ONBOARDING: 'd1b3e8f4-7a2e-4c9d-b5f1-9a8c6d2e4f3b',
} as const;

// USB HID Keyboard modifier bits
const MODIFIER_BITS = {
  LEFT_CTRL: 0x01,   // Bit 0
  LEFT_SHIFT: 0x02,  // Bit 1
  LEFT_ALT: 0x04,    // Bit 2
  LEFT_GUI: 0x08,    // Bit 3
  RIGHT_CTRL: 0x10,  // Bit 4
  RIGHT_SHIFT: 0x20, // Bit 5
  RIGHT_ALT: 0x40,   // Bit 6
  RIGHT_GUI: 0x80,   // Bit 7
} as const;

// Global modifier state management
let modifierBitmap: number = 0;

/**
 * Get the modifier bitmap with left/right distinction
 * @param event - KeyboardEvent
 * @param isPressed - true for keydown, false for keyup
 * @returns Updated modifier bitmap
 */
function updateModifierBitmap(event: KeyboardEvent, isPressed: boolean): number {
  let bitmap = modifierBitmap;

  if (isPressed) {
    // Update based on code to distinguish left/right
    switch (event.code) {
      case 'ControlLeft':
        bitmap |= MODIFIER_BITS.LEFT_CTRL;
        break;
      case 'ControlRight':
        bitmap |= MODIFIER_BITS.RIGHT_CTRL;
        break;
      case 'ShiftLeft':
        bitmap |= MODIFIER_BITS.LEFT_SHIFT;
        break;
      case 'ShiftRight':
        bitmap |= MODIFIER_BITS.RIGHT_SHIFT;
        break;
      case 'AltLeft':
        bitmap |= MODIFIER_BITS.LEFT_ALT;
        break;
      case 'AltRight':
        bitmap |= MODIFIER_BITS.RIGHT_ALT;
        break;
      case 'MetaLeft':
        bitmap |= MODIFIER_BITS.LEFT_GUI;
        break;
      case 'MetaRight':
        bitmap |= MODIFIER_BITS.RIGHT_GUI;
        break;
    }
  } else {
    // Clear bit on keyup
    switch (event.code) {
      case 'ControlLeft':
        bitmap &= ~MODIFIER_BITS.LEFT_CTRL;
        break;
      case 'ControlRight':
        bitmap &= ~MODIFIER_BITS.RIGHT_CTRL;
        break;
      case 'ShiftLeft':
        bitmap &= ~MODIFIER_BITS.LEFT_SHIFT;
        break;
      case 'ShiftRight':
        bitmap &= ~MODIFIER_BITS.RIGHT_SHIFT;
        break;
      case 'AltLeft':
        bitmap &= ~MODIFIER_BITS.LEFT_ALT;
        break;
      case 'AltRight':
        bitmap &= ~MODIFIER_BITS.RIGHT_ALT;
        break;
      case 'MetaLeft':
        bitmap &= ~MODIFIER_BITS.LEFT_GUI;
        break;
      case 'MetaRight':
        bitmap &= ~MODIFIER_BITS.RIGHT_GUI;
        break;
    }
  }

  modifierBitmap = bitmap;
  return bitmap;
}

// Reset modifier bitmap when window loses focus
window.addEventListener('blur', () => {
  modifierBitmap = 0;
});

// Markdown text to HTML converter (supports **bold** formatting)
function renderMarkdown(text: string): string {
  return text
    .split('\n')
    .map(line => {
      // Replace **text** with <strong>text</strong>
      return line.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    })
    .join('<br>');
}

// MIT License text
const MIT_LICENSE_TEXT = `**MIT License**

**Copyright (c) 2026 Nerry**

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

----

**Apps**:

**KL-01**

**川合堂ライセンス-01  ver.1.0  2000.12.30 H.Kawai (川合秀実)**

  川合秀実URL     http://k.osask.jp/
       e-mail     kawai@osask.jp

----

**CrystalCPUID for HariboteOS**

**KL-01**

Copyright (c) 2007 hiyohiyo (Project HiyOS) https://crystalmark.info/

----

**Icons**:

**Tabler Icons (https://tabler-icons.io/)**

**MIT License**

**Copyright (c) 2020-2026 Paweł Kuna**

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

----

**Web UI**:
Built with GitHub Copilot

----

**Special thanks to**:

* megumish🍕
* wataash
* NiwakaDev
`;

// File icon SVG data (src/assets/icons/)
const FILE_ICONS = {
  file: iconFile,
  fileText: iconFileText,
  appWindow: iconAppWindow,
  photo: iconPhoto,
  music: iconFileMusic,
};

const getFileIcon = (filename: string, pathExt: string): string => {
  const ext = filename.includes('.') ? '.' + filename.split('.').pop()!.toLowerCase() : '';
  const extensions = pathExt.split(':').filter(Boolean).map(e => e.toLowerCase());

  // Check if it's an executable file
  if (extensions.includes(ext)) {
    return FILE_ICONS.appWindow;
  }

  // Check if it's a text file
  if (ext === '.txt') {
    return FILE_ICONS.fileText;
  }

  // Check if it's an image file
  if (ext === '.bmp' || ext === '.jpg') {
    return FILE_ICONS.photo;
  }

  // Check if it's a music file
  if (ext === '.mml') {
    return FILE_ICONS.music;
  }

  // Default to generic file icon
  return FILE_ICONS.file;
};

const root = document.querySelector<HTMLDivElement>('#app');

if (!root) {
  throw new Error('Root element #app was not found.');
}

root.innerHTML = `
  <main class="app-shell">
    <section id="desktop" class="desktop" aria-label="Desktop"></section>
    <footer class="taskbar" aria-label="Taskbar">
      <div class="taskbar-left">
        <button id="hamburger-menu" type="button" class="hamburger-menu" aria-label="Menu"></button>
        <div id="menu-popup" class="menu-popup hidden" role="menu">
          <button id="menu-new-terminal" type="button" class="menu-item" role="menuitem">ターミナル</button>
          <button id="menu-file" type="button" class="menu-item" role="menuitem">ファイル</button>
          <button id="menu-about" type="button" class="menu-item" role="menuitem">About...</button>
        </div>
      </div>
      <div id="taskbar-center" class="taskbar-center" aria-label="Window buttons"></div>
      <div class="taskbar-actions">
        <div class="taskbar-volume-container">
          <button id="taskbar-volume-button" type="button" class="taskbar-volume-button" aria-label="Volume"></button>
          <div id="volume-popup" class="volume-popup hidden">
            <div class="volume-display" id="volume-display">50</div>
            <div class="volume-controls">
              <span class="volume-icon volume-icon-min">${iconVolume4}</span>
              <input id="volume-slider" type="range" class="volume-slider" min="0" max="100" value="50" aria-label="Volume slider">
              <span class="volume-icon volume-icon-max">${iconVolume}</span>
            </div>
          </div>
        </div>
        <div id="clock-display" class="clock-display" aria-live="polite"></div>
      </div>
    </footer>
  </main>
`;

const desktop = document.querySelector<HTMLDivElement>('#desktop');
const hamburgerMenu = document.querySelector<HTMLButtonElement>('#hamburger-menu');
const menuPopup = document.querySelector<HTMLDivElement>('#menu-popup');
const menuNewTerminal = document.querySelector<HTMLButtonElement>('#menu-new-terminal');
const menuFile = document.querySelector<HTMLButtonElement>('#menu-file');
const menuAbout = document.querySelector<HTMLButtonElement>('#menu-about');
const taskbarCenter = document.querySelector<HTMLDivElement>('#taskbar-center');
const clockDisplay = document.querySelector<HTMLDivElement>('#clock-display');
const taskbarVolumeButton = document.querySelector<HTMLButtonElement>('#taskbar-volume-button');
const volumePopup = document.querySelector<HTMLDivElement>('#volume-popup');
const volumeSlider = document.querySelector<HTMLInputElement>('#volume-slider');
const volumeDisplay = document.querySelector<HTMLDivElement>('#volume-display');

if (!desktop || !hamburgerMenu || !menuPopup || !menuNewTerminal || !menuFile || !menuAbout || !taskbarCenter || !clockDisplay || !taskbarVolumeButton || !volumePopup || !volumeSlider || !volumeDisplay) {
  throw new Error('Required UI elements could not be initialized.');
}

// Initialize start menu items with icons
menuNewTerminal.innerHTML = `<span class="menu-item-icon">${iconTerminal}</span><span class="menu-item-label">ターミナル</span>`;
menuFile.innerHTML = `<span class="menu-item-icon">${iconFolder}</span><span class="menu-item-label">ファイル</span>`;
menuAbout.innerHTML = `<span class="menu-item-icon">${iconCategory}</span><span class="menu-item-label">About...</span>`;

document.documentElement.style.setProperty('--taskbar-height', `${TASKBAR_HEIGHT_PX}px`);
document.documentElement.style.setProperty('--titlebar-height', `${TITLE_BAR_HEIGHT}px`);

const formatClock = (date: Date): string => {
  return new Intl.DateTimeFormat('ja-JP', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(date);
};

const updateClock = (): void => {
  clockDisplay.textContent = formatClock(new Date());
};

updateClock();
window.setInterval(updateClock, 1000);

const state: AppState = {
  windows: [],
  nextZIndex: 1,
  env: {
    PATH_EXT: '.hrb',
  },
  isMenuOpen: false,
  defaultTerminalWindowId: null,
  activeWindowId: null,
  windowGroups: [],
  oscillators: new Map(),
  globalVolume: 0.5,
  volumePopupVisible: false,
  audioContextState: 'suspended',
};

let activeDrag: DragState | null = null;
const canvasImageByWindow = new Map<WindowId, CanvasImage>();
const terminalOutputByWindow = new Map<WindowId, string[]>();
const terminalHistoryByWindow = new Map<WindowId, string[]>();
const terminalHistoryIndexByWindow = new Map<WindowId, number>();
const terminalPendingInputByWindow = new Map<WindowId, string>();
const fileSystem = new Map<string, FileEntry>();
const fileManagerSelectedIndexByWindowId = new Map<WindowId, number>();
const textViewerScrollByWindowId = new Map<WindowId, number>();
const fileManagerListScrollByWindowId = new Map<WindowId, number>();
let nextTerminalSpawnIndex = 0;
let hasShownStorageWarning = false;
const activeWorkers = new Set<Worker>();
const workerByWindowId = new Map<WindowId, Worker>();
const workerIdsByWorker = new Map<Worker, Set<string>>(); // Track workerId (for oscillators) by Worker
const tasksSkipDefaultTerminalFallback = new Set<WindowId>();
let fileManagerWindowId: WindowId | null = null;
let aboutWindowId: WindowId | null = null;
let onboardingWindowId: WindowId | null = null;
const textViewerWindowByFilename = new Map<string, WindowId>();
let lastFileManagerPosition: { x: number; y: number } | null = null;

// Audio system initialization and management
const VOLUME_STORAGE_KEY = 'haribote.audio.volume';

const initializeAudioContext = (): void => {
  if (state.audioContext) {
    return;
  }

  try {
    const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
    state.audioContext = audioContext;
    const gainNode = audioContext.createGain();
    gainNode.connect(audioContext.destination);
    gainNode.gain.value = state.globalVolume;
    state.globalGain = gainNode;
    state.audioContextState = audioContext.state as any;
    console.log(`[Audio] AudioContext initialized with state: ${audioContext.state}`);
  } catch (error) {
    console.error('[Audio] Failed to initialize AudioContext:', error);
  }
};

const resumeAudioContext = async (): Promise<void> => {
  if (!state.audioContext) {
    initializeAudioContext();
  }

  if (!state.audioContext || state.audioContext.state === 'running') {
    return;
  }

  try {
    await state.audioContext.resume();
    state.audioContextState = 'running';
    updateVolumeIcon();
    console.log('[Audio] AudioContext resumed');
  } catch (error) {
    console.error('[Audio] Failed to resume AudioContext:', error);
  }
};

const updateVolumeIcon = (): void => {
  if (!taskbarVolumeButton) {
    return;
  }

  let iconHtml: string;

  // Show icon based on volume level (regardless of AudioContext state)
  const volumePercent = state.globalVolume * 100;
  if (volumePercent === 0) {
    // Muted (0%)
    iconHtml = iconVolume3;
  } else if (volumePercent >= 70) {
    // Large (9~10)
    iconHtml = iconVolume;
  } else if (volumePercent >= 30) {
    // Medium (4~8)
    iconHtml = iconVolume2;
  } else {
    // Small (1~3)
    iconHtml = iconVolume4;
  }

  taskbarVolumeButton.innerHTML = iconHtml;
};

const stopOscillator = (workerId: string): void => {
  const oscillatorData = state.oscillators.get(workerId);
  if (!oscillatorData) {
    return;
  }

  try {
    oscillatorData.osc.stop();
    oscillatorData.gain.disconnect();
  } catch (error) {
    console.warn(`[Audio] Error stopping oscillator for worker ${workerId}:`, error);
  }

  state.oscillators.delete(workerId);
};

const playSound = (workerId: string, frequency: number, timestamp: number): void => {
  if (!state.audioContext) {
    initializeAudioContext();
  }

  if (!state.audioContext || !state.globalGain) {
    return;
  }

  // Resume AudioContext if suspended
  if (state.audioContext.state === 'suspended') {
    resumeAudioContext();
  }

  // Stop existing oscillator for this worker
  stopOscillator(workerId);

  // If frequency is 0, just stop (already done above)
  if (frequency <= 0) {
    return;
  }

  try {
    const osc = state.audioContext.createOscillator();
    const gain = state.audioContext.createGain();

    osc.frequency.value = frequency / 1000;
    osc.type = 'square';
    osc.connect(gain);
    gain.connect(state.globalGain);
    gain.gain.value = 0.15; // Reduced from 0.3 to lower square wave volume

    osc.start(state.audioContext.currentTime);
    state.oscillators.set(workerId, {
      osc,
      gain,
      startTime: state.audioContext.currentTime,
      workerTimestamp: timestamp,
    });

    // console.log(`[Audio] Started oscillator for worker ${workerId} at ${frequency}Hz (worker timestamp: ${timestamp}ms)`);
  } catch (error) {
    console.error(`[Audio] Error playing sound for worker ${workerId}:`, error);
  }
};

const setGlobalVolume = (volume: number): void => {
  const clampedVolume = Math.max(0, Math.min(1, volume));
  state.globalVolume = clampedVolume;

  if (state.globalGain && state.audioContext) {
    state.globalGain.gain.value = clampedVolume;
  }

  // Save to localStorage
  localStorage.setItem(VOLUME_STORAGE_KEY, String(clampedVolume));
  updateVolumeIcon();
};

const handleVolumeSliderChange = (e: Event): void => {
  const slider = e.target as HTMLInputElement;
  const volumeValue = parseInt(slider.value, 10);
  const volume = volumeValue / 100;
  setGlobalVolume(volume);
  // Update the display value
  volumeDisplay.textContent = String(volumeValue);
};

const handleVolumeButtonClick = (e: MouseEvent): void => {
  e.stopPropagation();
  state.volumePopupVisible = !state.volumePopupVisible;

  if (state.volumePopupVisible) {
    volumePopup.classList.remove('hidden');
    const volumePercent = Math.round(state.globalVolume * 100);
    volumeSlider.value = String(volumePercent);
    volumeDisplay.textContent = String(volumePercent);
    // Try to resume AudioContext
    resumeAudioContext();
  } else {
    volumePopup.classList.add('hidden');
  }
};

const closeVolumePopup = (): void => {
  state.volumePopupVisible = false;
  volumePopup.classList.add('hidden');
};

// Load saved volume from localStorage
const savedVolume = localStorage.getItem(VOLUME_STORAGE_KEY);
if (savedVolume) {
  try {
    const volume = parseFloat(savedVolume);
    if (!isNaN(volume) && volume >= 0 && volume <= 1) {
      state.globalVolume = volume;
    }
  } catch {
    // Ignore parsing errors, use default
  }
}

// Set up event listeners for volume control
taskbarVolumeButton.addEventListener('click', handleVolumeButtonClick);
volumeSlider.addEventListener('change', handleVolumeSliderChange);
volumeSlider.addEventListener('input', handleVolumeSliderChange);

// Initialize volume icon on startup
updateVolumeIcon();

// Close volume popup when clicking outside
document.addEventListener('click', (e: MouseEvent) => {
  if (state.volumePopupVisible && !taskbarVolumeButton.contains(e.target as Node) && !volumePopup.contains(e.target as Node)) {
    closeVolumePopup();
  }
});

const createWindowId = (): WindowId => {
  return crypto.randomUUID();
};

const toBase64 = (bytes: Uint8Array): string => {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
};

const fromBase64 = (base64: string): Uint8Array => {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
};

/**
 * ウィンドウグループ化関数：ウィンドウタイプごとにグループ化情報を計算
 * Canvas タイプは個別表示、その他のタイプはグループ化
 */
const groupWindowsByKind = (windows: WindowModel[]): WindowGroupInfo[] => {
  const grouped = new Map<string, WindowModel[]>();
  const canvasWindows: WindowModel[] = [];

  for (const win of windows) {
    if (win.kind === 'canvas') {
      canvasWindows.push(win);
    } else {
      const key = win.kind;
      if (!grouped.has(key)) {
        grouped.set(key, []);
      }
      grouped.get(key)!.push(win);
    }
  }

  const groups: WindowGroupInfo[] = [];

  // 非Canvas ウィンドウをグループ化
  for (const [kind, windowList] of grouped) {
    const appId = windowList[0].appId;
    groups.push({
      appId,
      kind: kind as WindowKind,
      windowIds: windowList.map(w => w.id),
      isExpanded: false,
    });
  }

  // Canvas ウィンドウは個別表示
  for (const canvasWin of canvasWindows) {
    groups.push({
      appId: canvasWin.appId,
      kind: 'canvas',
      windowIds: [canvasWin.id],
      isExpanded: false,
    });
  }

  return groups;
};


const getStoredFileSystemV1 = (): StoredFileSystemV1 => {
  return {
    version: 1,
    files: [...fileSystem.values()]
      .filter((entry) => !entry.isInitialFile)
      .map((entry) => ({
        name: entry.name,
        contentBase64: toBase64(entry.content),
      })),
  };
};

const emitGlobalTerminalLine = (message: string): void => {
  for (const windowId of terminalOutputByWindow.keys()) {
    const current = terminalOutputByWindow.get(windowId) ?? [];
    current.push(message);
    terminalOutputByWindow.set(windowId, current);
    syncTerminalView(windowId);
  }
};

const applyInitialFiles = (): void => {
  for (const entry of INITIAL_FS_ENTRIES) {
    const compressed = fromBase64(entry.contentBase64);
    const decompressed = inflate(compressed);
    fileSystem.set(toCanonicalFileKey(entry.name), {
      name: entry.name,
      content: decompressed,
      isInitialFile: true,
    });
  }
};

const mergeStoredFiles = (files: Array<{ name: string; contentBase64: string }>): void => {
  for (const entry of files) {
    if (!entry || typeof entry.name !== 'string' || typeof entry.contentBase64 !== 'string') {
      continue;
    }

    fileSystem.set(toCanonicalFileKey(entry.name), {
      name: entry.name,
      content: fromBase64(entry.contentBase64),
      isInitialFile: false,
    });
  }
};

const persistFileSystem = (): void => {
  const storedV1 = getStoredFileSystemV1();
  const compressedPayload = deflate(TEXT_ENCODER.encode(JSON.stringify(storedV1)));
  const serialized = toBase64(compressedPayload);
  const serializedSize = TEXT_ENCODER.encode(serialized).length;

  try {
    localStorage.setItem(FILE_STORAGE_KEY, serialized);
  } catch {
    emitGlobalTerminalLine('Filesystem save failed: localStorage quota exceeded.');
    return;
  }

  if (serializedSize > FILE_STORAGE_WARNING_BYTES) {
    const warning = `Warning: filesystem uses ${serializedSize} bytes (> ${FILE_STORAGE_WARNING_BYTES} bytes).`;
    emitGlobalTerminalLine(warning);
    console.warn(warning);
    if (!hasShownStorageWarning) {
      hasShownStorageWarning = true;
      window.alert(`${warning}\nPlease remove unnecessary files.`);
    }
  }
};

const loadFileSystem = (): void => {
  fileSystem.clear();

  // 1. Initialize with initial files
  applyInitialFiles();

  // 2. Merge with localStorage data
  const raw = localStorage.getItem(FILE_STORAGE_KEY);
  if (!raw) {
    return;
  }

  try {
    const compressed = fromBase64(raw);
    const decompressedBytes = inflate(compressed);
    const decompressedJson = TEXT_DECODER.decode(decompressedBytes);
    const parsed = JSON.parse(decompressedJson) as StoredFileSystemV1;
    if (parsed.version === 1 && Array.isArray(parsed.files)) {
      mergeStoredFiles(parsed.files);
    }
  } catch {
    localStorage.removeItem(FILE_STORAGE_KEY);
  }
};

/**
 * ファイルシステムのバリデーション・正規化ルール
 * 
 * 許可される文字: A-Z, a-z, 0-9, '.', '_', '!'
 * 最大ファイル名長: 31文字（初期FSとの互換性）
 * 
 * バリデーション処理:
 * 1. パス区切り文字 (/ \) を除去してファイル名のみ抽出
 * 2. 非ASCII文字を下線に置換
 * 3. 非ASCII文字が50%以上の場合は処理失敗
 * 4. 連続する下線をまとめ、先頭・末尾のドットを削除（パストラバーサル対策）
 * 5. ファイル名長が31文字を超える場合はトリミング（拡張子を優先保持）
 */
const normalizePathLikeName = (input: string): string => {
  return input.split(/[\\/]/).filter(Boolean).at(-1) ?? '';
};

const toCanonicalFileKey = (name: string): string => {
  return name.toUpperCase();
};

type NormalizeFileNameResult = { ok: true; name: string } | { ok: false; reason: string };

// 最大ファイル名長（初期FSとの互換性）
const MAX_FILE_NAME_LENGTH = 31;
// 名前が長い場合のトリミング時に保持する最小ベース名長
const MIN_BASE_NAME_LENGTH = 8;

const splitBaseAndExtension = (name: string): { base: string; ext: string } => {
  const dotIndex = name.lastIndexOf('.');
  // ドットが先頭にある、または末尾にある場合は拡張子として扱わない
  if (dotIndex <= 0 || dotIndex === name.length - 1) {
    return { base: name, ext: '' };
  }

  return {
    base: name.slice(0, dotIndex),
    ext: name.slice(dotIndex + 1),
  };
};

// ファイル名のサニタイズ
// 許可文字：英数字、アンダースコア、ドット、感嘆符
// 非ASCII文字は下線に置換
const sanitizeAsciiFileName = (name: string): { sanitized: string; nonAsciiCount: number } => {
  let nonAsciiCount = 0;
  let sanitized = '';

  for (const char of name) {
    const code = char.charCodeAt(0);
    if (code > 127) {
      nonAsciiCount += 1;
      sanitized += '_';
      continue;
    }

    // 許可文字: A-Z, a-z, 0-9, '.', '_', '!'
    if (/[A-Za-z0-9_.!]/.test(char)) {
      sanitized += char;
      continue;
    }

    sanitized += '_';
  }

  return { sanitized, nonAsciiCount };
};

// ファイル名の正規化と検証
// 1. パス区切り文字を除去
// 2. 許可文字以外を下線に置換
// 3. 非ASCII文字が50%以上でないかチェック
// 4. 連続する下線をまとめ、先頭・末尾のドットを削除
// 5. 長さが31文字を超える場合はトリミング
const normalizeFileName = (input: string): NormalizeFileNameResult => {
  const raw = normalizePathLikeName(input.trim());
  if (!raw) {
    return { ok: false, reason: 'Filename is empty.' };
  }

  const { sanitized, nonAsciiCount } = sanitizeAsciiFileName(raw);
  // 非ASCII文字が50%以上含まれている場合は拒否
  if (nonAsciiCount > 0 && nonAsciiCount * 2 > raw.length) {
    return {
      ok: false,
      reason: `Filename conversion failed: too many non-ASCII characters in ${raw}`,
    };
  }

  // 連続下線をまとめ、先頭・末尾のドットを除去（パストラバーサル対策）
  const collapsed = sanitized.replace(/_+/g, '_').replace(/^\.+/, '').replace(/\.+$/, '');
  if (!collapsed) {
    return { ok: false, reason: `Filename conversion failed: ${raw}` };
  }

  const { base: rawBase, ext: rawExt } = splitBaseAndExtension(collapsed);
  const base = rawBase || '_';
  const ext = rawExt;

  if (collapsed.length <= MAX_FILE_NAME_LENGTH) {
    return { ok: true, name: collapsed };
  }

  // ファイル名が長すぎる場合のトリミング処理
  if (!ext) {
    // 拡張子がない場合はベース名だけをトリミング
    return { ok: true, name: base.slice(0, MAX_FILE_NAME_LENGTH) };
  }

  // 拡張子が長くない場合は拡張子を保持してベース名をトリミング
  const baseLimitWhenKeepingFullExt = MAX_FILE_NAME_LENGTH - 1 - ext.length;
  if (baseLimitWhenKeepingFullExt >= MIN_BASE_NAME_LENGTH) {
    return {
      ok: true,
      name: `${base.slice(0, baseLimitWhenKeepingFullExt)}.${ext}`,
    };
  }

  // ベース名の最小長も満たせない場合は拡張子もトリミング
  const extLimit = Math.max(0, MAX_FILE_NAME_LENGTH - 1 - MIN_BASE_NAME_LENGTH);
  return {
    ok: true,
    name: `${base.slice(0, MIN_BASE_NAME_LENGTH)}.${ext.slice(0, extLimit)}`,
  };
};

const formatBytes = (size: number): string => {
  return `${size.toLocaleString('ja-JP')}`;
};

const upsertFile = (rawName: string, content: Uint8Array): { ok: true; name: string } | { ok: false; reason: string } => {
  const normalized = normalizeFileName(rawName);
  if (!normalized.ok) {
    return { ok: false, reason: normalized.reason };
  }

  const name = normalized.name;
  const key = toCanonicalFileKey(name);

  fileSystem.set(key, { name, content, isInitialFile: false });
  persistFileSystem();
  return { ok: true, name };
};

const removeFile = (rawName: string): { ok: true; name: string } | { ok: false; reason: string } => {
  const normalized = normalizeFileName(rawName);
  if (!normalized.ok) {
    return { ok: false, reason: normalized.reason };
  }

  const name = normalized.name;
  const key = toCanonicalFileKey(name);
  if (!fileSystem.has(key)) {
    return { ok: false, reason: `File not found: ${rawName}` };
  }
  fileSystem.delete(key);
  persistFileSystem();
  return { ok: true, name };
};

const renameFile = (
  rawSource: string,
  rawDestination: string,
): { ok: true; source: string; destination: string } | { ok: false; reason: string } => {
  const sourceNormalized = normalizeFileName(rawSource);
  if (!sourceNormalized.ok) {
    return { ok: false, reason: sourceNormalized.reason };
  }

  const destinationNormalized = normalizeFileName(rawDestination);
  if (!destinationNormalized.ok) {
    return { ok: false, reason: destinationNormalized.reason };
  }

  const source = sourceNormalized.name;
  const destination = destinationNormalized.name;
  const sourceKey = toCanonicalFileKey(source);
  const destinationKey = toCanonicalFileKey(destination);
  const sourceEntry = fileSystem.get(sourceKey);

  if (!sourceEntry) {
    return { ok: false, reason: `File not found: ${rawSource}` };
  }

  fileSystem.delete(sourceKey);
  fileSystem.set(destinationKey, {
    name: destination,
    content: sourceEntry.content,
    isInitialFile: false,
  });
  persistFileSystem();

  return { ok: true, source, destination };
};

const createWindowModel = (windowModel: WindowModel): void => {
  state.windows = state.windows.map((item) => ({
    ...item,
    isActive: false,
  }));
  state.windows.push(windowModel);
};

const findWindowById = (id: WindowId): WindowModel | undefined => {
  return state.windows.find((item) => item.id === id);
};

const syncFramePosition = (id: WindowId): void => {
  const target = findWindowById(id);
  if (!target) {
    return;
  }

  const frame = desktop.querySelector<HTMLElement>(`[data-window-id="${id}"]`);
  if (!frame) {
    return;
  }

  frame.style.left = `${target.x}px`;
  frame.style.top = `${target.y}px`;
};

const getTopZIndex = (): number => {
  if (state.windows.length === 0) {
    return 0;
  }

  return Math.max(...state.windows.map((item) => item.zIndex));
};

const refreshZIndices = (): void => {
  const sorted = [...state.windows].sort((a, b) => a.zIndex - b.zIndex);
  const nextById = new Map<WindowId, number>();

  sorted.forEach((win, index) => {
    nextById.set(win.id, index + 1);
  });

  state.windows = state.windows.map((win) => ({
    ...win,
    zIndex: nextById.get(win.id) ?? win.zIndex,
  }));
  state.nextZIndex = state.windows.length + 1;
};

const refreshZIndicesIfNeeded = (): void => {
  if (state.nextZIndex <= Z_INDEX_REFRESH_THRESHOLD) {
    return;
  }

  refreshZIndices();
};

const clamp = (value: number, min: number, max: number): number => {
  if (value < min) {
    return min;
  }
  if (value > max) {
    return max;
  }
  return value;
};

const clampWindowToDesktop = (win: WindowModel): WindowModel => {
  return win;
};

const clampPointerToDesktop = (clientX: number, clientY: number): { x: number; y: number } => {
  const rect = desktop.getBoundingClientRect();
  return {
    x: clamp(clientX, rect.left, rect.right),
    y: clamp(clientY, rect.top, rect.bottom),
  };
};

const getCenteredWindowPosition = (width: number, height: number): { x: number; y: number } => {
  const rect = desktop.getBoundingClientRect();
  const desktopWidth = Math.max(0, Math.floor(rect.width));
  const desktopHeight = Math.max(0, Math.floor(rect.height));

  return {
    x: Math.max(0, Math.floor((desktopWidth - width) / 2)),
    y: Math.max(0, Math.floor((desktopHeight - height) / 2)),
  };
};

const bringToFrontIfNeeded = (id: WindowId): void => {
  const target = state.windows.find((item) => item.id === id);
  if (!target) {
    return;
  }

  const maxZIndex = getTopZIndex();
  const shouldRaise = target.zIndex < maxZIndex;
  const shouldActivate = !target.isActive;

  if (!shouldRaise && !shouldActivate) {
    return;
  }

  state.activeWindowId = id;
  state.windows = state.windows.map((item) => {
    if (item.id !== id) {
      return {
        ...item,
        isActive: false,
      };
    }

    return {
      ...item,
      zIndex: shouldRaise ? state.nextZIndex : item.zIndex,
      isActive: true,
    };
  });

  if (shouldRaise) {
    state.nextZIndex += 1;
    refreshZIndicesIfNeeded();
  }

  renderWindows();

  // Focus file manager list if this is a file manager window
  if (target.kind === 'filemanager') {
    focusFileManagerList(id);
  } else if (target.kind === 'terminal') {
    focusTerminalInput(id);
  }
};

const focusTerminalInput = (windowId: WindowId): void => {
  const input = desktop.querySelector<HTMLInputElement>(`[data-terminal-input-window-id="${windowId}"]`);
  input?.focus({ preventScroll: true });
};

const isInteractiveContentTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  return Boolean(target.closest('input, button, textarea, select, form'));
};

const closeMenu = (): void => {
  state.isMenuOpen = false;
  menuPopup.classList.add('hidden');
};

const toggleMenu = (): void => {
  state.isMenuOpen = !state.isMenuOpen;
  if (state.isMenuOpen) {
    menuPopup.classList.remove('hidden');
  } else {
    menuPopup.classList.add('hidden');
  }
};

const createTerminalWindow = (): void => {
  const id = createWindowId();
  const offset = nextTerminalSpawnIndex * TERMINAL_WINDOW_OFFSET_STEP;

  const windowModel: WindowModel = clampWindowToDesktop({
    id,
    appId: APP_IDS.TERMINAL,
    kind: 'terminal',
    title: 'Terminal',
    x: INITIAL_TERMINAL_X + offset,
    y: INITIAL_TERMINAL_Y + offset,
    width: INITIAL_TERMINAL_WINDOW_WIDTH,
    height: INITIAL_TERMINAL_WINDOW_HEIGHT,
    zIndex: state.nextZIndex,
    isActive: true,
  });

  nextTerminalSpawnIndex += 1;
  state.nextZIndex += 1;
  createWindowModel(windowModel);
  terminalOutputByWindow.set(id, []);
  runTerminalCommand(id, 'VER', { echoInput: false, skipHistory: true });

  // Set as default terminal if none is set
  if (state.defaultTerminalWindowId === null) {
    state.defaultTerminalWindowId = id;
  }

  renderWindows();
  requestAnimationFrame(() => {
    focusTerminalInput(id);
  });
};

const openRustWindow = (windowId: WindowId, width: number, height: number, title: string): void => {
  const existing = findWindowById(windowId);
  if (existing) {
    bringToFrontIfNeeded(windowId);
    return;
  }

  const actualWidth = Math.max(TITLE_BAR_HEIGHT * 4, width);
  const actualHeight = Math.max(TITLE_BAR_HEIGHT + 20, height);
  const centered = getCenteredWindowPosition(actualWidth, actualHeight);

  const windowModel: WindowModel = {
    id: windowId,
    appId: APP_IDS.CANVAS,
    kind: 'canvas',
    title,
    x: centered.x,
    y: centered.y,
    width: actualWidth,
    height: actualHeight,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  // console.log('[openRustWindow] Creating new window model', windowModel, width, height);

  state.nextZIndex += 1;
  state.activeWindowId = windowId;
  createWindowModel(windowModel);
  renderWindows();
};

const closeWindow = (id: WindowId): void => {
  // Notify Worker if closing a Canvas window
  const closingWindow = findWindowById(id);
  if (closingWindow && closingWindow.kind === 'canvas') {
    const worker = workerByWindowId.get(id);
    if (worker) {
      const closeMessage: WindowCloseMessage = {
        type: 'windowClose',
        windowId: id,
      };
      worker.postMessage(closeMessage);
      workerByWindowId.delete(id);
    }
  }

  canvasImageByWindow.delete(id);
  terminalOutputByWindow.delete(id);
  terminalHistoryByWindow.delete(id);
  terminalHistoryIndexByWindow.delete(id);
  terminalPendingInputByWindow.delete(id);
  fileManagerSelectedIndexByWindowId.delete(id);
  fileManagerListScrollByWindowId.delete(id);
  textViewerScrollByWindowId.delete(id);

  // Update default terminal if the closed window was the default
  if (state.defaultTerminalWindowId === id) {
    const remainingTerminals = state.windows
      .filter((item) => item.kind === 'terminal' && item.id !== id)
      .sort((a, b) => {
        // Get the order they were created
        const terminalA = Array.from(terminalOutputByWindow.keys()).indexOf(a.id);
        const terminalB = Array.from(terminalOutputByWindow.keys()).indexOf(b.id);
        return terminalA - terminalB;
      });
    state.defaultTerminalWindowId = remainingTerminals[0]?.id ?? null;
  }

  // Clear file manager window ID if it was closed
  if (fileManagerWindowId === id) {
    const winModel = findWindowById(id);
    if (winModel) {
      lastFileManagerPosition = { x: winModel.x, y: winModel.y };
    }
    fileManagerWindowId = null;
  }

  // Clear about window ID if it was closed
  if (aboutWindowId === id) {
    aboutWindowId = null;
  }

  // Clear onboarding window ID if it was closed
  if (onboardingWindowId === id) {
    onboardingWindowId = null;
  }

  // Clear text viewer window IDs if they were closed
  for (const [filename, windowId] of textViewerWindowByFilename.entries()) {
    if (windowId === id) {
      textViewerWindowByFilename.delete(filename);
    }
  }

  const remainingWindows = state.windows.filter((item) => item.id !== id);

  if (remainingWindows.length === 0) {
    state.windows = [];
    renderWindows();
    return;
  }

  const topWindow = remainingWindows.reduce((top, current) => {
    return current.zIndex > top.zIndex ? current : top;
  });

  state.windows = remainingWindows.map((item) => ({
    ...item,
    isActive: item.id === topWindow.id,
  }));

  renderWindows();
};

const updateWindowPosition = (id: WindowId, x: number, y: number): void => {
  state.windows = state.windows.map((item) => {
    if (item.id !== id) {
      return item;
    }

    return clampWindowToDesktop({ ...item, x, y: Math.max(0, y) });
  });
};

const setWindowPositionById = (id: WindowId, x: number, y: number): void => {
  updateWindowPosition(id, x, y);
  syncFramePosition(id);
};

const drawImageOnCanvas = (id: WindowId, image: CanvasImage): void => {
  const canvas = desktop.querySelector<HTMLCanvasElement>(`[data-canvas-window-id="${id}"]`);
  if (!canvas) {
    return;
  }

  const ctx = canvas.getContext('2d');
  if (!ctx) {
    return;
  }

  if (image.width <= 0 || image.height <= 0) {
    return;
  }

  const expectedLength = image.width * image.height * 4;
  if (image.pixels.byteLength !== expectedLength) {
    console.log(`[drawImageOnCanvas] Pixel data length mismatch: expected ${expectedLength}, got ${image.pixels.byteLength}`);
    return;
  }

  const pixels = new Uint8ClampedArray(image.pixels);
  const imageData = new ImageData(pixels, image.width, image.height);
  ctx.putImageData(imageData, image.x, image.y);

  // ctx.strokeStyle = 'red';
  // ctx.strokeRect(image.x, image.y, image.width, image.height);
};

const syncTerminalView = (windowId: WindowId): void => {
  const output = terminalOutputByWindow.get(windowId) ?? [];
  const screen = desktop.querySelector<HTMLPreElement>(`[data-terminal-screen-window-id="${windowId}"]`);
  if (!screen) {
    return;
  }

  screen.textContent = output.join('\n');
  screen.scrollTop = screen.scrollHeight;
};

const appendTerminalText = (windowId: WindowId, text: string): void => {
  const current = terminalOutputByWindow.get(windowId) ?? [];

  // Split text by newlines
  const parts = text.split('\n');

  for (let i = 0; i < parts.length; i++) {
    if (i === 0) {
      // First part: append to the last line (or create new if empty)
      if (current.length === 0) {
        current.push(parts[i]);
      } else {
        current[current.length - 1] += parts[i];
      }
    } else {
      // Subsequent parts: each becomes a new line
      current.push(parts[i]);
    }
  }

  // Remove old lines if we exceed the maximum
  if (current.length > TERMINAL_MAX_LINES) {
    current.splice(0, current.length - TERMINAL_MAX_LINES);
  }

  terminalOutputByWindow.set(windowId, current);
  syncTerminalView(windowId);
};

const appendTerminalLine = (windowId: WindowId, text: string): void => {
  appendTerminalText(windowId, `${text}\n`);
};

const splitCommandTokens = (inputText: string): string[] => {
  return inputText.trim().split(/\s+/).filter(Boolean);
};

const decodeFileAsText = (entry: FileEntry): string => {
  try {
    // First, try to detect the encoding
    const detectedEnc = Encoding.detect(entry.content);

    // If Shift_JIS is detected, try that first
    if (detectedEnc === 'SJIS') {
      try {
        const converted = Encoding.convert(entry.content, { from: 'SJIS', to: 'UNICODE' });
        if (Array.isArray(converted)) {
          return String.fromCharCode(...converted);
        }
      } catch { }
    }

    // Try UTF-8
    try {
      return TEXT_DECODER.decode(entry.content);
    } catch { }

    // Fallback: Try Shift_JIS even if not detected
    try {
      const converted = Encoding.convert(entry.content, { from: 'SJIS', to: 'UNICODE' });
      if (Array.isArray(converted)) {
        return String.fromCharCode(...converted);
      }
    } catch { }

    // Last resort: Use UTF-8 with error replacement
    return TEXT_DECODER.decode(entry.content);
  } catch {
    return '[Error decoding file]';
  }
};

const runTerminalCommand = (windowId: WindowId, inputText: string, options?: { echoInput?: boolean; skipHistory?: boolean }): void => {
  const normalized = inputText.trim();
  if (!normalized) {
    return;
  }

  // 履歴に追加（skipHistory が false でない場合）
  if (!options?.skipHistory) {
    if (!terminalHistoryByWindow.has(windowId)) {
      terminalHistoryByWindow.set(windowId, []);
    }

    const history = terminalHistoryByWindow.get(windowId)!;

    // 重複排除：同じコマンドが履歴にあれば古い方を削除
    const existingIndex = history.indexOf(normalized);
    if (existingIndex !== -1) {
      history.splice(existingIndex, 1);
    }

    history.unshift(normalized);
  }

  terminalHistoryIndexByWindow.set(windowId, -1);
  terminalPendingInputByWindow.delete(windowId);

  if (options?.echoInput !== false) {
    appendTerminalLine(windowId, `> ${inputText}`);
  }

  const tokens = splitCommandTokens(normalized);
  const command = tokens[0]?.toUpperCase() ?? '';
  const args = tokens.slice(1);
  const argsText = normalized.slice(tokens[0]?.length ?? 0).trimStart();

  switch (command) {
    case 'VER':
      appendTerminalLine(windowId, `${APP_NAME} v${__APP_VERSION__} (${__GIT_HASH__})`);
      return;
    case 'ECHO':
      appendTerminalLine(windowId, argsText);
      return;
    case 'CLS': {
      terminalOutputByWindow.set(windowId, []);
      syncTerminalView(windowId);
      return;
    }
    case 'DIR':
    case 'LS': {
      const entries = [...fileSystem.values()].sort((a, b) => a.name.localeCompare(b.name));
      if (entries.length === 0) {
        appendTerminalLine(windowId, 'No files.');
        return;
      }

      let totalSize = 0;
      for (const entry of entries) {
        totalSize += entry.content.byteLength;
        appendTerminalLine(windowId, `${entry.name.padEnd(13, ' ')} ${formatBytes(entry.content.byteLength)}`);
      }
      appendTerminalLine(windowId, `  ${entries.length} file(s), total ${formatBytes(totalSize)} bytes`);
      return;
    }
    case 'TYPE': {
      const targetName = args[0] ?? '';
      if (!targetName) {
        appendTerminalLine(windowId, 'Usage: TYPE <filename>');
        return;
      }

      const normalizedResult = normalizeFileName(targetName);
      if (!normalizedResult.ok) {
        appendTerminalLine(windowId, normalizedResult.reason);
        return;
      }

      const normalizedName = normalizedResult.name;
      const entry = fileSystem.get(toCanonicalFileKey(normalizedName));
      if (!entry) {
        appendTerminalLine(windowId, `File not found: ${targetName}`);
        return;
      }

      const text = decodeFileAsText(entry);
      if (!text) {
        appendTerminalLine(windowId, '(empty file)');
        return;
      }

      for (const line of text.split(/\r?\n/)) {
        appendTerminalLine(windowId, line);
      }
      return;
    }
    case 'COPY': {
      const sourceRaw = args[0] ?? '';
      const destinationRaw = args[1] ?? '';
      if (!sourceRaw || !destinationRaw) {
        appendTerminalLine(windowId, 'Usage: COPY <source> <destination>');
        return;
      }

      const sourceNormalized = normalizeFileName(sourceRaw);
      if (!sourceNormalized.ok) {
        appendTerminalLine(windowId, sourceNormalized.reason);
        return;
      }

      const sourceName = sourceNormalized.name;
      const sourceEntry = fileSystem.get(toCanonicalFileKey(sourceName));
      if (!sourceEntry) {
        appendTerminalLine(windowId, `File not found: ${sourceRaw}`);
        return;
      }

      const result = upsertFile(destinationRaw, new Uint8Array(sourceEntry.content));
      if (!result.ok) {
        appendTerminalLine(windowId, result.reason);
        return;
      }

      appendTerminalLine(windowId, `${sourceName} copied to ${result.name}`);
      return;
    }
    case 'DEL': {
      const targetRaw = args[0] ?? '';
      if (!targetRaw) {
        appendTerminalLine(windowId, `Usage: DEL <filename>`);
        return;
      }

      const result = removeFile(targetRaw);
      if (!result.ok) {
        appendTerminalLine(windowId, result.reason);
        return;
      }

      appendTerminalLine(windowId, `Deleted ${result.name}`);
      return;
    }
    case 'REN': {
      const sourceRaw = args[0] ?? '';
      const destinationRaw = args[1] ?? '';
      if (!sourceRaw || !destinationRaw) {
        appendTerminalLine(windowId, `Usage: REN <source> <destination>`);
        return;
      }

      const result = renameFile(sourceRaw, destinationRaw);
      if (!result.ok) {
        appendTerminalLine(windowId, result.reason);
        return;
      }

      appendTerminalLine(windowId, `${result.source} renamed to ${result.destination}`);
      return;
    }
    case 'EXIT': {
      closeWindow(windowId);
      return;
    }
    case 'SET': {
      const setArg = argsText.trim();
      if (!setArg) {
        // Display all environment variables
        const entries = Object.entries(state.env).sort(([a], [b]) => a.localeCompare(b));
        if (entries.length === 0) {
          appendTerminalLine(windowId, '(no environment variables set)');
        } else {
          for (const [name, value] of entries) {
            appendTerminalLine(windowId, `${name}=${value}`);
          }
        }
        return;
      }

      const eqIndex = setArg.indexOf('=');
      if (eqIndex === -1) {
        // Display current environment variable value
        const varName = setArg.trim();
        const value = state.env[varName];
        if (value !== undefined) {
          appendTerminalLine(windowId, `${varName}=${value}`);
        } else {
          appendTerminalLine(windowId, `${varName} is not set`);
        }
        return;
      }

      // Set environment variable
      const varName = setArg.slice(0, eqIndex).trim();
      const value = setArg.slice(eqIndex + 1).trim();

      if (!varName) {
        appendTerminalLine(windowId, 'Variable name cannot be empty');
        return;
      }

      state.env[varName] = value;
      appendTerminalLine(windowId, `${varName}=${value}`);
      return;
    }
    case 'HELP': {
      // Display help for selected commands (cognitive load mitigation)
      appendTerminalLine(windowId, 'Available Commands:');
      appendTerminalLine(windowId, '');
      appendTerminalLine(windowId, 'VER              - Display app version and git hash');
      appendTerminalLine(windowId, 'DIR              - List files with size information');
      appendTerminalLine(windowId, 'TYPE <filename>  - Display file contents as text');
      appendTerminalLine(windowId, 'NCST <file>      - Execute file with no output display');
      return;
    }
    case 'START':
    case 'NCST':
    case 'OPEN': {
      // Launch Rust task with dummy terminal (no output display)
      // Errors are only logged to console.error
      launchNoDisplayTask(normalized, tokens);
      return;
    }
    default: {
      const commandToken = tokens[0] ?? '';
      const normalizedFile = normalizeFileName(commandToken);
      if (normalizedFile.ok) {
        const entry = fileSystem.get(toCanonicalFileKey(normalizedFile.name));
        if (entry) {
          launchRustTaskWithCommand(windowId, entry.name, normalized);
          return;
        }
      }

      // Try with PATH_EXT extensions
      const pathExt = state.env['PATH_EXT'] ?? '.hrb';
      const extensions = pathExt.split(':').filter(Boolean);

      for (const ext of extensions) {
        const fileNameWithExt = commandToken + ext;
        const normalizedWithExt = normalizeFileName(fileNameWithExt);
        if (normalizedWithExt.ok) {
          const entry = fileSystem.get(toCanonicalFileKey(normalizedWithExt.name));
          if (entry) {
            launchRustTaskWithCommand(windowId, entry.name, normalized);
            return;
          }
        }
      }

      appendTerminalLine(windowId, 'Bad command or file name');
      return;
    }
  }
};

const broadcastFileSystemSnapshot = (): void => {
  const fileSystemSnapshot = [...fileSystem.values()].map((entry) => ({
    name: entry.name,
    content: entry.content.slice().buffer,
  }));

  const message: UpdateFileSystemSnapshotMessage = {
    type: 'updateFileSystemSnapshot',
    fileSystemSnapshot,
  };

  for (const worker of activeWorkers) {
    worker.postMessage(message);
  }
};

const handleWorkerCommand = (command: WorkerCommand, worker?: Worker): void => {
  switch (command.type) {
    case 'openWindow':
      openRustWindow(command.windowId, command.width, command.height, command.title);
      // Register Canvas window to Worker mapping when window is created
      if (worker) {
        workerByWindowId.set(command.windowId, worker);
      }
      return;
    case 'moveWindow':
      setWindowPositionById(command.windowId, command.x, command.y);
      return;
    case 'activateWindow':
      bringToFrontIfNeeded(command.windowId);
      return;
    case 'closeWindow':
      closeWindow(command.windowId);
      return;
    case 'drawImage': {
      // Get window to determine canvas size
      const win = findWindowById(command.windowId);
      if (!win) {
        return;
      }

      const canvasWidth = win.width;
      const canvasHeight = win.height - TITLE_BAR_HEIGHT;

      // Get or create full canvas buffer
      let fullBuffer = canvasImageByWindow.get(command.windowId);
      if (!fullBuffer || fullBuffer.width !== canvasWidth || fullBuffer.height !== canvasHeight) {
        // Create new buffer if doesn't exist or size changed
        fullBuffer = {
          x: 0,
          y: 0,
          width: canvasWidth,
          height: canvasHeight,
          pixels: new ArrayBuffer(canvasWidth * canvasHeight * 4),
        };
        // Fill with default background color (#f2f8f7)
        const bytes = new Uint8Array(fullBuffer.pixels);
        for (let i = 0; i < bytes.length; i += 4) {
          bytes[i] = 0xF2;
          bytes[i + 1] = 0xF8;
          bytes[i + 2] = 0xF7;
          bytes[i + 3] = 0xFF;
        }
      }

      // Update buffer with partial image data
      const bufferBytes = new Uint8Array(fullBuffer.pixels);
      const newPixels = new Uint8Array(command.pixels);
      for (let y = 0; y < command.height; y++) {
        const bufferOffset = ((command.y + y) * canvasWidth + command.x) * 4;
        const pixelOffset = y * command.width * 4;
        bufferBytes.set(
          newPixels.subarray(pixelOffset, pixelOffset + command.width * 4),
          bufferOffset,
        );
      }

      // Save updated buffer
      canvasImageByWindow.set(command.windowId, fullBuffer);

      // Draw the partial update to canvas
      const partialImage: CanvasImage = {
        x: command.x,
        y: command.y,
        width: command.width,
        height: command.height,
        pixels: command.pixels,
      };
      drawImageOnCanvas(command.windowId, partialImage);
      return;
    }
    case 'print':
      if (terminalOutputByWindow.has(command.windowId)) {
        appendTerminalText(command.windowId, command.text);
      } else if (state.defaultTerminalWindowId && terminalOutputByWindow.has(state.defaultTerminalWindowId) && !tasksSkipDefaultTerminalFallback.has(command.windowId)) {
        // Fallback to default terminal if specified window doesn't exist (unless this is a no-display task)
        appendTerminalText(state.defaultTerminalWindowId, command.text);
      } else {
        // Ultimate fallback: to do nothing for now
      }
      return;
    case 'println':
      if (terminalOutputByWindow.has(command.windowId)) {
        appendTerminalLine(command.windowId, command.text);
      } else if (state.defaultTerminalWindowId && terminalOutputByWindow.has(state.defaultTerminalWindowId) && !tasksSkipDefaultTerminalFallback.has(command.windowId)) {
        // Fallback to default terminal if specified window doesn't exist (unless this is a no-display task)
        appendTerminalLine(state.defaultTerminalWindowId, command.text);
      } else {
        // Ultimate fallback: to do nothing for now
      }
      return;
    case 'writeFile': {
      const normalizedResult = normalizeFileName(command.filename);
      if (!normalizedResult.ok) {
        return;
      }
      const key = toCanonicalFileKey(normalizedResult.name);
      const exists = fileSystem.has(key);
      if (command.mode === WriteFileMode.Update && !exists) {
        return;
      }
      if (command.mode === WriteFileMode.Create && exists) {
        return;
      }
      fileSystem.set(key, { name: normalizedResult.name, content: new Uint8Array(command.data), isInitialFile: false });
      persistFileSystem();
      // Broadcast updated file system snapshot to all active workers
      broadcastFileSystemSnapshot();
      return;
    }
    case 'playSound': {
      // Use the workerId from the worker to track oscillators
      playSound(command.workerId, command.frequency, command.timestamp);
      // Record this workerId for cleanup when the worker terminates
      if (worker) {
        if (!workerIdsByWorker.has(worker)) {
          workerIdsByWorker.set(worker, new Set());
        }
        workerIdsByWorker.get(worker)!.add(command.workerId);
      }
      return;
    }
    case 'error':
      console.error(`Rust worker error: ${command.message}`);
      return;
    case 'done':
      return;
  }
};

const startRustWorker = (startMessage: WorkerStartMessage): void => {
  const worker = new Worker(new URL('./rustTask.worker.ts', import.meta.url), { type: 'module' });
  activeWorkers.add(worker);

  worker.addEventListener('message', (event: MessageEvent<WorkerCommand>) => {
    handleWorkerCommand(event.data, worker);
    if (event.data.type === 'done' || event.data.type === 'error') {
      activeWorkers.delete(worker);
      // Clean up all Oscillators created by this worker
      const workerIds = workerIdsByWorker.get(worker);
      if (workerIds) {
        for (const workerId of workerIds) {
          stopOscillator(workerId);
        }
        workerIdsByWorker.delete(worker);
        console.log(`[Audio] Cleaned up ${workerIds.size} oscillator(s) for terminated worker`);
      }
      // Clean up all Canvas window mappings for this worker
      for (const [windowId, w] of workerByWindowId.entries()) {
        if (w === worker) {
          workerByWindowId.delete(windowId);
        }
      }
      worker.terminate();
    }
  });
  worker.addEventListener('error', (event) => {
    const errorMsg = `Worker error: ${event.error?.message ?? event.message ?? 'Unknown error'}`;
    console.error(errorMsg, event.error);

    // Also output to default terminal if available for visibility in Chromium
    if (state.defaultTerminalWindowId && terminalOutputByWindow.has(state.defaultTerminalWindowId)) {
      appendTerminalLine(state.defaultTerminalWindowId, `[worker error] ${errorMsg}`);
    }

    activeWorkers.delete(worker);
    // Clean up all Canvas window mappings for this worker
    for (const [windowId, w] of workerByWindowId.entries()) {
      if (w === worker) {
        workerByWindowId.delete(windowId);
      }
    }
    worker.terminate();
  });

  try {
    // console.log('[main] Posting message to worker...');
    worker.postMessage(startMessage);
    // console.log('[main] Message posted successfully');
  } catch (error) {
    const errorMsg = `Failed to start worker: ${error instanceof Error ? error.message : String(error)}`;
    console.error(errorMsg);
    if (state.defaultTerminalWindowId && terminalOutputByWindow.has(state.defaultTerminalWindowId)) {
      appendTerminalLine(state.defaultTerminalWindowId, `[worker] ${errorMsg}`);
    }
    activeWorkers.delete(worker);
    // Clean up all Canvas window mappings for this worker
    for (const [windowId, w] of workerByWindowId.entries()) {
      if (w === worker) {
        workerByWindowId.delete(windowId);
      }
    }
    worker.terminate();
  }
};

const launchNoDisplayTask = (normalizedInput: string, tokens: string[]): void => {
  if (tokens.length < 2) {
    console.error('Usage: <command> <filename> [args...]');
    return;
  }

  // Extract command line after command name (without command name itself)
  const afterCommandIndex = normalizedInput.indexOf(tokens[1]);
  const commandLine = normalizedInput.slice(afterCommandIndex);

  const commandToken = tokens[1];
  const normalizedFile = normalizeFileName(commandToken);
  if (normalizedFile.ok) {
    const entry = fileSystem.get(toCanonicalFileKey(normalizedFile.name));
    if (entry) {
      const dummyWindowId = createWindowId();
      tasksSkipDefaultTerminalFallback.add(dummyWindowId);
      launchRustTaskWithCommand(dummyWindowId, entry.name, commandLine);
      return;
    }
  }

  // Try with PATH_EXT extensions
  const pathExt = state.env['PATH_EXT'] ?? '.hrb';
  const extensions = pathExt.split(':').filter(Boolean);

  for (const ext of extensions) {
    const fileNameWithExt = commandToken + ext;
    const normalizedWithExt = normalizeFileName(fileNameWithExt);
    if (normalizedWithExt.ok) {
      const entry = fileSystem.get(toCanonicalFileKey(normalizedWithExt.name));
      if (entry) {
        const dummyWindowId = createWindowId();
        tasksSkipDefaultTerminalFallback.add(dummyWindowId);
        launchRustTaskWithCommand(dummyWindowId, entry.name, commandLine);
        return;
      }
    }
  }

  console.error('Bad command or file name');
};

const searchExecutableFile = (fileName: string, pathExt: string): string | null => {
  const normalizedFile = normalizeFileName(fileName);
  if (normalizedFile.ok) {
    const entry = fileSystem.get(toCanonicalFileKey(normalizedFile.name));
    if (entry) {
      return entry.name;
    }
  }

  // Try with PATH_EXT extensions
  const extensions = pathExt.split(':').filter(Boolean);
  for (const ext of extensions) {
    const fileNameWithExt = fileName + ext;
    const normalizedWithExt = normalizeFileName(fileNameWithExt);
    if (normalizedWithExt.ok) {
      const entry = fileSystem.get(toCanonicalFileKey(normalizedWithExt.name));
      if (entry) {
        return entry.name;
      }
    }
  }

  return null;
};

const launchRustTaskWithCommand = (terminalWindowId: WindowId, fileName: string, commandLine: string): void => {
  const fileSystemSnapshot = [...fileSystem.values()].map((entry) => ({
    name: entry.name,
    content: entry.content.slice().buffer,
  }));

  startRustWorker({
    type: 'startWithCommand',
    seed: (Math.random() * 0xffffffff) >>> 0,
    titleBarHeight: TITLE_BAR_HEIGHT,
    terminalWindowId,
    fileName,
    commandLine,
    fileSystemSnapshot,
    environmentVariables: state.env,
  });
};

const startDrag = (event: PointerEvent, windowModel: WindowModel): void => {
  const clampedPointer = clampPointerToDesktop(event.clientX, event.clientY);

  activeDrag = {
    id: windowModel.id,
    pointerId: event.pointerId,
    startClientX: clampedPointer.x,
    startClientY: clampedPointer.y,
    originX: windowModel.x,
    originY: windowModel.y,
  };

  window.addEventListener('pointermove', handlePointerMove);
  window.addEventListener('pointerup', endDrag);
  window.addEventListener('pointercancel', endDrag);
};

const handlePointerMove = (event: PointerEvent): void => {
  if (!activeDrag || activeDrag.pointerId !== event.pointerId) {
    return;
  }

  const clampedPointer = clampPointerToDesktop(event.clientX, event.clientY);
  const diffX = clampedPointer.x - activeDrag.startClientX;
  const diffY = clampedPointer.y - activeDrag.startClientY;
  updateWindowPosition(activeDrag.id, activeDrag.originX + diffX, activeDrag.originY + diffY);
  syncFramePosition(activeDrag.id);
};

const endDrag = (event: PointerEvent): void => {
  if (!activeDrag || activeDrag.pointerId !== event.pointerId) {
    return;
  }

  activeDrag = null;
  window.removeEventListener('pointermove', handlePointerMove);
  window.removeEventListener('pointerup', endDrag);
  window.removeEventListener('pointercancel', endDrag);
};

const createCanvas = (win: WindowModel): HTMLCanvasElement => {
  const canvas = document.createElement('canvas');
  const canvasWidth = win.width;
  const canvasHeight = win.height - TITLE_BAR_HEIGHT;
  canvas.className = 'window-canvas';
  canvas.dataset.canvasWindowId = win.id;
  canvas.width = Math.max(1, canvasWidth);
  canvas.height = Math.max(1, canvasHeight);
  canvas.style.width = `${Math.max(1, canvasWidth)}px`;
  canvas.style.height = `${Math.max(1, canvasHeight)}px`;

  const ctx = canvas.getContext('2d');
  if (ctx) {
    ctx.fillStyle = '#f2f8f7';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    const image = canvasImageByWindow.get(win.id);
    if (image && image.width === canvas.width && image.height === canvas.height) {
      const pixels = new Uint8ClampedArray(image.pixels);
      const imageData = new ImageData(pixels, image.width, image.height);
      ctx.putImageData(imageData, 0, 0);
    }
  }

  return canvas;
};

const createFileManagerPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'filemanager-panel';

  const list = document.createElement('div');
  list.className = 'filemanager-list';
  list.dataset.fileManagerListWindowId = win.id;

  // Restore selected index from map, default to 0
  let selectedIndex = fileManagerSelectedIndexByWindowId.get(win.id) ?? 0;
  let lastClickTime = 0;

  const renderList = (): void => {
    // Save current scroll position before clearing
    const savedScroll = list.scrollTop;
    list.textContent = '';
    const files = [...fileSystem.values()].sort((a, b) => a.name.localeCompare(b.name));

    if (files.length === 0) {
      const emptyMsg = document.createElement('div');
      emptyMsg.className = 'filemanager-empty';
      emptyMsg.textContent = '(no files)';
      list.appendChild(emptyMsg);
      return;
    }

    let selectedItem: HTMLElement | null = null;

    files.forEach((file, index) => {
      const item = document.createElement('div');
      item.className = 'filemanager-item';
      if (index === selectedIndex) {
        item.classList.add('filemanager-item-selected');
        // Add data attribute to indicate active state when rendered
        item.dataset.isSelected = 'true';
        selectedItem = item;
      }
      item.dataset.fileIndex = String(index);

      // Create icon element
      const iconContainer = document.createElement('span');
      iconContainer.className = 'filemanager-item-icon';
      iconContainer.innerHTML = getFileIcon(file.name, state.env['PATH_EXT'] ?? '.hrb');

      // Create filename element
      const nameContainer = document.createElement('span');
      nameContainer.className = 'filemanager-item-name';
      nameContainer.textContent = file.name;

      item.appendChild(iconContainer);
      item.appendChild(nameContainer);

      item.addEventListener('mousedown', (event) => {
        event.stopPropagation();
        event.preventDefault();
        list.focus({ preventScroll: true });
      });

      item.addEventListener('click', (event) => {
        event.stopPropagation();
        event.preventDefault();
        const now = Date.now();
        const isDoubleClick = now - lastClickTime < 300;
        lastClickTime = now;

        if (isDoubleClick) {
          executeFileFromFileManager(index);
        } else {
          selectedIndex = index;
          fileManagerSelectedIndexByWindowId.set(win.id, selectedIndex);
          renderList();
        }
      });

      list.appendChild(item);
    });

    // Auto-scroll to keep selected item in view
    if (selectedItem) {
      (selectedItem as HTMLElement).scrollIntoView({ block: 'nearest', behavior: 'auto' });
    }

    // Restore scroll position if it was saved
    const storedScroll = fileManagerListScrollByWindowId.get(win.id);
    if (storedScroll !== undefined) {
      requestAnimationFrame(() => {
        list.scrollTop = storedScroll;
      });
    } else if (savedScroll > 0) {
      // Keep the previously saved scroll position from before the clear
      requestAnimationFrame(() => {
        list.scrollTop = savedScroll;
      });
    }
  };

  const handleKeydown = (event: KeyboardEvent): void => {
    // Only handle keyboard input if this file manager window is active
    if (state.activeWindowId !== win.id) {
      return;
    }

    const files = [...fileSystem.values()].sort((a, b) => a.name.localeCompare(b.name));

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      event.stopPropagation();
      selectedIndex = Math.min(selectedIndex + 1, files.length - 1);
      fileManagerSelectedIndexByWindowId.set(win.id, selectedIndex);
      renderList();
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      event.stopPropagation();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      fileManagerSelectedIndexByWindowId.set(win.id, selectedIndex);
      renderList();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      event.stopPropagation();
      if (selectedIndex >= 0) {
        executeFileFromFileManager(selectedIndex);
      }
    } else if (['ArrowLeft', 'ArrowRight', ' '].includes(event.key)) {
      // Prevent other arrow keys and space from causing scroll
      event.preventDefault();
      event.stopPropagation();
    }
  };

  const handleKeyup = (event: KeyboardEvent): void => {
    // Prevent default for keys that might cause scroll
    if (['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight', ' '].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
    }
  };

  list.addEventListener('keydown', handleKeydown);
  list.addEventListener('keyup', handleKeyup);
  list.addEventListener('scroll', () => {
    fileManagerListScrollByWindowId.set(win.id, list.scrollTop);
  });
  list.addEventListener('focusout', () => {
    // Re-focus the list when focus is lost (but not when switching windows)
    if (state.activeWindowId === win.id) {
      setTimeout(() => list.focus({ preventScroll: true }), 0);
    }
  });
  list.addEventListener('click', () => {
    // Ensure focus is retained on click
    list.focus({ preventScroll: true });
  });
  list.setAttribute('tabindex', '0');
  list.setAttribute('role', 'listbox');
  list.style.outline = 'none';

  panel.appendChild(list);
  renderList();

  return panel;
};

const createOnboardingPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'onboarding-panel';
  panel.style.backgroundImage = `url(${blissImage})`;
  panel.style.backgroundSize = 'cover';
  panel.style.backgroundPosition = 'center';
  panel.style.backgroundRepeat = 'no-repeat';

  // Container for content
  const contentContainer = document.createElement('div');
  contentContainer.className = 'onboarding-content';

  // Title
  const title = document.createElement('h2');
  title.className = 'onboarding-title';
  title.textContent = 'ようこそ！';
  contentContainer.appendChild(title);

  // Description
  const description = document.createElement('p');
  description.className = 'onboarding-description';
  description.innerHTML = '<a href="http://hrb.osask.jp/" target="_blank" rel="noopener noreferrer">はりぼてOS</a> のアプリをブラウザー上で実行できるデスクトップ環境エミュレーターです。';
  contentContainer.appendChild(description);

  // Features list
  const features = document.createElement('ul');
  features.className = 'onboarding-features';

  const featuresList = [
    '一般的なウィンドウシステムと同様にウィンドウ操作できます。',
    'ファイル一覧から HRB ファイルを選択してGUIアプリを実行できます。',
    '一部のアプリはターミナルでコマンド入力が必要なものがあります。',
    'ターミナルでは HELP コマンドで使用可能なコマンドを確認できます。',
    'ドラッグアンドドロップで外部の HRB ファイルを取り込めます。',
  ];

  featuresList.forEach(feature => {
    const li = document.createElement('li');
    li.className = 'onboarding-feature-item';
    li.textContent = feature;
    features.appendChild(li);
  });

  contentContainer.appendChild(features);

  const betaNotice = document.createElement('p');
  betaNotice.className = 'onboarding-description';
  betaNotice.textContent = '現在ベータ運用中です。未実装機能がたくさんあります。';
  contentContainer.appendChild(betaNotice);

  // // Tips section
  // const tipsSection = document.createElement('div');
  // tipsSection.className = 'onboarding-tips';

  // const tipsTitle = document.createElement('h3');
  // tipsTitle.textContent = 'ヒント';
  // tipsSection.appendChild(tipsTitle);

  // const tipsList = document.createElement('ul');
  // tipsList.className = 'onboarding-tips-list';

  // const tips = [
  //   'ターミナルでは HELP コマンドで使用可能なコマンドを確認できます。',
  // ];

  // tips.forEach(tip => {
  //   const li = document.createElement('li');
  //   li.className = 'onboarding-tip-item';
  //   li.textContent = tip;
  //   tipsList.appendChild(li);
  // });

  // tipsSection.appendChild(tipsList);
  // contentContainer.appendChild(tipsSection);

  // Button container
  const buttonContainer = document.createElement('div');
  buttonContainer.className = 'onboarding-button-container';

  const closeButton = document.createElement('button');
  closeButton.type = 'button';
  closeButton.textContent = '開始する';
  closeButton.className = 'onboarding-close-button window-primary-button';
  closeButton.addEventListener('click', (event) => {
    event.stopPropagation();
    closeWindow(win.id);
  });

  buttonContainer.appendChild(closeButton);
  contentContainer.appendChild(buttonContainer);

  panel.appendChild(contentContainer);
  return panel;
};

const createAboutPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'about-panel';

  // App info container
  const infoContainer = document.createElement('div');
  infoContainer.className = 'about-info';

  const appNameDiv = document.createElement('div');
  appNameDiv.className = 'about-app-name';
  appNameDiv.innerHTML = '<strong>HariboteBox</strong>';
  infoContainer.appendChild(appNameDiv);

  const versionDiv = document.createElement('div');
  versionDiv.className = 'about-version';
  versionDiv.textContent = `v${__APP_VERSION__} | ${__GIT_HASH__}`;
  infoContainer.appendChild(versionDiv);

  // License text container
  const licenseContainer = document.createElement('div');
  licenseContainer.className = 'about-license-container';

  const licenseLabel = document.createElement('label');
  licenseLabel.htmlFor = 'about-license-text';
  licenseLabel.textContent = 'License:';

  const licenseDiv = document.createElement('div');
  licenseDiv.id = 'about-license-text';
  licenseDiv.className = 'about-license-text';
  licenseDiv.innerHTML = renderMarkdown(MIT_LICENSE_TEXT);
  licenseDiv.setAttribute('role', 'textbox');
  licenseDiv.setAttribute('aria-label', 'MIT License');
  licenseDiv.style.whiteSpace = 'pre-wrap';
  licenseDiv.style.wordWrap = 'break-word';
  licenseDiv.style.overflowY = 'auto';
  licenseDiv.style.overflowX = 'hidden';
  licenseDiv.style.padding = '8px';

  licenseContainer.append(licenseLabel, licenseDiv);

  // OK button container
  const buttonContainer = document.createElement('div');
  buttonContainer.className = 'about-button-container';

  const githubButton = document.createElement('button');
  githubButton.type = 'button';
  githubButton.textContent = 'GitHub';
  githubButton.className = 'about-github-button window-secondary-button';
  githubButton.addEventListener('click', (event) => {
    event.stopPropagation();
    window.open('https://github.com/neri/haribox/', '_blank');
  });

  const okButton = document.createElement('button');
  okButton.type = 'button';
  okButton.textContent = 'OK';
  okButton.className = 'about-ok-button window-primary-button';
  okButton.addEventListener('click', (event) => {
    event.stopPropagation();
    closeWindow(win.id);
  });

  buttonContainer.append(githubButton, okButton);

  panel.append(infoContainer, licenseContainer, buttonContainer);
  return panel;
};

const executeFileFromFileManager = (index: number): void => {
  const files = [...fileSystem.values()].sort((a, b) => a.name.localeCompare(b.name));
  if (index < 0 || index >= files.length) {
    return;
  }

  const file = files[index];
  const ext = file.name.includes('.') ? '.' + file.name.split('.').pop()!.toLowerCase() : '';
  const pathExt = state.env['PATH_EXT'] ?? '.hrb';
  const extensions = pathExt.split(':').filter(Boolean).map(e => e.toLowerCase());

  // Check if file can be executed
  if (extensions.includes(ext)) {
    // Launch as task
    launchRustTaskWithCommand(state.defaultTerminalWindowId ?? '', file.name, file.name);
    return;
  }

  // Check if it's an image file
  if (ext === '.bmp' || ext === '.jpg') {
    // Search for gview file
    const gviewSearch = searchExecutableFile('gview', pathExt);
    if (gviewSearch) {
      launchRustTaskWithCommand(state.defaultTerminalWindowId ?? '', gviewSearch, `gview ${file.name}`);
    } else {
      showErrorDialog(`このファイルは実行できません: gview`);
    }
    return;
  }

  // Check if it's a text file
  if (ext === '.txt') {
    // Search for tview file
    const tviewSearch = searchExecutableFile('tview', pathExt);
    if (tviewSearch) {
      launchRustTaskWithCommand(state.defaultTerminalWindowId ?? '', tviewSearch, `tview -w80 -h24 ${file.name}`);
    } else {
      showErrorDialog(`このファイルは実行できません: tview`);
    }
    return;
  }

  // Check if it's a music file
  if (ext === '.mml') {
    // Resume audio context for playback
    resumeAudioContext();

    // Search for mmlplay file
    const mmlplaySearch = searchExecutableFile('mmlplay', pathExt);
    if (mmlplaySearch) {
      launchRustTaskWithCommand(state.defaultTerminalWindowId ?? '', mmlplaySearch, `mmlplay ${file.name}`);
    } else {
      showErrorDialog(`このファイルは実行できません: mmlplay`);
    }
    return;
  }

  // Show error dialog
  showErrorDialog(`このファイルは実行できません: ${file.name}`);
};

const showErrorDialog = (message: string): void => {
  window.alert(message);
};

const createTextViewerPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'textviewer-panel';

  const textContainer = document.createElement('div');
  textContainer.className = 'textviewer-container';

  const textArea = document.createElement('textarea');
  textArea.className = 'textviewer-textarea';
  textArea.readOnly = true;
  textArea.setAttribute('aria-label', 'File content');
  textArea.setAttribute('aria-readonly', 'true');

  // Extract filename from window title
  const filename = win.title;

  // Read file content and populate textarea
  const entry = fileSystem.get(toCanonicalFileKey(filename));
  if (entry) {
    try {
      const content = decodeFileAsText(entry);
      textArea.value = content;
    } catch (error) {
      textArea.value = `[Error decoding file: ${filename}]`;
      console.error(`Failed to decode file ${filename}:`, error);
    }
  } else {
    textArea.value = `[File not found: ${filename}]`;
  }

  // Restore scroll position if it was saved
  const savedScrollTop = textViewerScrollByWindowId.get(win.id);
  if (savedScrollTop !== undefined) {
    requestAnimationFrame(() => {
      textArea.scrollTop = savedScrollTop;
    });
  }

  // Save scroll position when user scrolls
  textArea.addEventListener('scroll', () => {
    textViewerScrollByWindowId.set(win.id, textArea.scrollTop);
  });

  // Activate window when textarea is clicked/focused
  textArea.addEventListener('pointerdown', (event) => {
    event.stopPropagation();
    if (!win.isActive) {
      bringToFrontIfNeeded(win.id);
      // Focus after render completes
      requestAnimationFrame(() => {
        const textarea = document.querySelector(`textarea[data-textviewer-textarea-window-id="${win.id}"]`) as HTMLTextAreaElement;
        textarea?.focus({ preventScroll: true });
      });
    } else {
      textArea.focus({ preventScroll: true });
    }
  });

  textArea.setAttribute('data-textviewer-textarea-window-id', win.id);

  textContainer.appendChild(textArea);
  panel.appendChild(textContainer);

  return panel;
};

const createTerminalPanel = (win: WindowModel): HTMLElement => {
  const panel = document.createElement('section');
  panel.className = 'terminal-panel';

  const screen = document.createElement('pre');
  screen.className = 'terminal-screen';
  screen.dataset.terminalScreenWindowId = win.id;
  screen.setAttribute('aria-live', 'polite');
  screen.textContent = (terminalOutputByWindow.get(win.id) ?? []).join('\n');

  const commandForm = document.createElement('form');
  commandForm.className = 'terminal-command-form';

  const commandInput = document.createElement('input');
  commandInput.className = 'terminal-command-input';
  commandInput.dataset.terminalInputWindowId = win.id;
  commandInput.type = 'text';
  commandInput.placeholder = 'Command';
  commandInput.setAttribute('aria-label', 'Command input');
  commandInput.addEventListener('focus', (event) => {
    event.preventDefault();
  });

  // Activate window and focus input when tapped in inactive terminal
  commandInput.addEventListener('pointerdown', (event) => {
    event.stopPropagation();
    if (!win.isActive) {
      bringToFrontIfNeeded(win.id);
      // Focus after render completes
      requestAnimationFrame(() => {
        const input = document.querySelector(`input[data-terminal-input-window-id="${win.id}"]`) as HTMLInputElement;
        input?.focus({ preventScroll: true });
      });
    } else {
      commandInput.focus({ preventScroll: true });
    }
  });

  // キーボード処理：上下キーで履歴操作
  commandInput.addEventListener('keydown', (event) => {
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      const history = terminalHistoryByWindow.get(win.id) ?? [];
      if (history.length === 0) return;

      let currentIndex = terminalHistoryIndexByWindow.get(win.id) ?? -1;

      // 初回ブラウズ時に入力途中のテキストを保存
      if (currentIndex === -1) {
        terminalPendingInputByWindow.set(win.id, commandInput.value);
      }

      const nextIndex = Math.min(currentIndex + 1, history.length - 1);
      terminalHistoryIndexByWindow.set(win.id, nextIndex);
      commandInput.value = history[nextIndex];
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      const history = terminalHistoryByWindow.get(win.id) ?? [];
      let currentIndex = terminalHistoryIndexByWindow.get(win.id) ?? -1;

      if (currentIndex > -1) {
        const nextIndex = currentIndex - 1;
        terminalHistoryIndexByWindow.set(win.id, nextIndex);

        // 履歴から抜けて入力中に戻る場合、保存されたテキストを復元
        if (nextIndex === -1) {
          commandInput.value = terminalPendingInputByWindow.get(win.id) ?? '';
        } else {
          commandInput.value = history[nextIndex];
        }
      }
    }
  });

  const executeButton = document.createElement('button');
  executeButton.className = 'terminal-command-button window-primary-button';
  executeButton.type = 'submit';
  executeButton.textContent = '実行';

  commandForm.addEventListener('submit', (event) => {
    event.preventDefault();
    runTerminalCommand(win.id, commandInput.value);
    commandInput.value = '';
    commandInput.focus({ preventScroll: true });
  });

  commandForm.append(commandInput, executeButton);
  panel.append(screen, commandForm);

  requestAnimationFrame(() => {
    screen.scrollTop = screen.scrollHeight;
  });

  return panel;
};

const getWindowIconSvg = (kind: WindowKind): string => {
  switch (kind) {
    case 'terminal':
      return iconTerminal;
    case 'filemanager':
      return iconFolder;
    case 'about':
      return iconCategory;
    case 'textviewer':
      return iconFileText;
    case 'canvas':
    default:
      return iconAppWindow;
  }
};

/**
 * タスクバーボタンをレンダリング
 */
const renderTaskbarButtons = (): void => {
  taskbarCenter.innerHTML = '';

  // 既存のドロップダウンメニューをクリーンアップ
  document.querySelectorAll('.taskbar-dropdown-menu').forEach(el => el.remove());

  state.windowGroups.forEach((group) => {
    const groupContainer = document.createElement('div');
    groupContainer.className = 'taskbar-button-group';

    // グループボタン（Canvas は個別表示なので、非Canvas のみグループ化）
    const isCanvasGroup = group.kind === 'canvas';
    const isGrouped = group.windowIds.length > 1 && !isCanvasGroup;

    const groupButton = document.createElement('button');
    groupButton.type = 'button';
    groupButton.className = 'taskbar-button';

    // アクティブウィンドウがこのグループに属するかチェック
    const isGroupActive = state.activeWindowId && group.windowIds.includes(state.activeWindowId);
    if (isGroupActive) {
      groupButton.classList.add('taskbar-button-active');
    }

    const iconSpan = document.createElement('span');
    iconSpan.className = 'taskbar-button-icon';
    iconSpan.innerHTML = getWindowIconSvg(group.kind);
    groupButton.appendChild(iconSpan);

    if (isGrouped) {
      // グループボタン表示
      const labelSpan = document.createElement('span');
      labelSpan.className = 'taskbar-button-label';
      labelSpan.textContent = `${getWindowTypeLabel(group.kind)} ×${group.windowIds.length}`;
      groupButton.appendChild(labelSpan);

      groupButton.addEventListener('click', () => {
        group.isExpanded = !group.isExpanded;
        renderTaskbarButtons();
      });

      groupContainer.appendChild(groupButton);

      // ドロップダウンメニュー
      if (group.isExpanded) {
        const dropdown = document.createElement('div');
        dropdown.className = 'taskbar-dropdown-menu';

        group.windowIds.forEach((windowId) => {
          const win = findWindowById(windowId);
          if (!win) return;

          const item = document.createElement('button');
          item.type = 'button';
          item.className = 'taskbar-dropdown-item';
          if (win.isActive) {
            item.classList.add('taskbar-dropdown-item-active');
          }
          item.textContent = win.title;

          item.addEventListener('click', () => {
            bringToFrontIfNeeded(windowId);
            group.isExpanded = false;
            renderTaskbarButtons();
          });

          dropdown.appendChild(item);
        });

        // ドロップダウンメニューを body に追加（fixed positioning用）
        document.body.appendChild(dropdown);

        // DOM に追加された後、位置を計算
        // (offsetHeight がまだ計算されていないことがあるため、setTimeout で遅延実行)
        setTimeout(() => {
          const groupButtonRect = groupButton.getBoundingClientRect();
          dropdown.style.position = 'fixed';
          dropdown.style.left = `${groupButtonRect.left}px`;
          dropdown.style.top = `${groupButtonRect.top - dropdown.offsetHeight - 4}px`;
        }, 0);
      }
    } else {
      // 個別ボタン表示
      const win = findWindowById(group.windowIds[0]);
      if (win) {
        // アクティブウィンドウかチェック
        if (win.isActive) {
          groupButton.classList.add('taskbar-button-active');
        }

        const labelSpan = document.createElement('span');
        labelSpan.className = 'taskbar-button-label';
        labelSpan.textContent = win.title;
        groupButton.appendChild(labelSpan);

        groupButton.addEventListener('click', () => {
          bringToFrontIfNeeded(win.id);
        });
      }

      groupContainer.appendChild(groupButton);
    }

    taskbarCenter.appendChild(groupContainer);
  });
};

const getWindowTypeLabel = (kind: WindowKind): string => {
  switch (kind) {
    case 'terminal':
      return 'Terminal';
    case 'filemanager':
      return 'File System';
    case 'about':
      return 'About';
    case 'textviewer':
      return 'Text Viewer';
    case 'canvas':
    default:
      return 'Task';
  }
};


const renderWindows = (): void => {
  // Save scroll position before rendering to prevent unwanted scrolling
  const scrollX = window.scrollX;
  const scrollY = window.scrollY;

  // ウィンドウグループを再計算してタスクバーボタンを更新
  state.windowGroups = groupWindowsByKind(state.windows);
  renderTaskbarButtons();

  desktop.textContent = '';

  const sortedWindows = [...state.windows].sort((a, b) => a.zIndex - b.zIndex);
  for (const win of sortedWindows) {
    const frame = document.createElement('article');
    frame.className = 'window-frame';
    if (win.isActive) {
      frame.classList.add('window-frame-active');
    }
    frame.dataset.windowId = win.id;
    frame.style.left = `${win.x}px`;
    frame.style.top = `${win.y}px`;
    frame.style.width = `${win.width}px`;
    frame.style.height = `${win.height}px`;
    frame.style.zIndex = String(win.zIndex);

    frame.addEventListener('pointerdown', (event) => {
      if (isInteractiveContentTarget(event.target)) {
        return;
      }

      bringToFrontIfNeeded(win.id);
    });

    const titleBar = document.createElement('header');
    titleBar.className = 'window-titlebar';
    titleBar.addEventListener('pointerdown', (event) => {
      if (event.button !== 0) {
        return;
      }

      bringToFrontIfNeeded(win.id);

      const latestWindow = findWindowById(win.id);
      if (!latestWindow) {
        return;
      }

      startDrag(event, latestWindow);
      event.preventDefault();
    });

    const title = document.createElement('span');
    title.className = 'window-title';
    title.textContent = win.title;

    const icon = document.createElement('span');
    icon.className = 'window-icon';
    icon.innerHTML = getWindowIconSvg(win.kind);

    const closeButton = document.createElement('button');
    closeButton.className = 'window-close-button';
    closeButton.type = 'button';
    closeButton.innerHTML = iconClose;
    closeButton.setAttribute('aria-label', `Close ${win.title}`);
    closeButton.addEventListener('pointerdown', (event) => {
      event.stopPropagation();
      event.preventDefault();
      closeWindow(win.id);
    });
    closeButton.addEventListener('click', (event) => {
      event.stopPropagation();
      closeWindow(win.id);
    });

    titleBar.append(icon, title, closeButton);
    frame.append(titleBar);
    if (win.kind === 'terminal') {
      frame.append(createTerminalPanel(win));
    } else if (win.kind === 'filemanager') {
      frame.append(createFileManagerPanel(win));
    } else if (win.kind === 'about') {
      frame.append(createAboutPanel(win));
    } else if (win.kind === 'onboarding') {
      frame.append(createOnboardingPanel(win));
    } else if (win.kind === 'textviewer') {
      frame.append(createTextViewerPanel(win));
    } else {
      frame.append(createCanvas(win));
    }
    desktop.appendChild(frame);
  }

  // Focus the appropriate element in the active window after rendering
  if (state.activeWindowId) {
    const activeWindow = findWindowById(state.activeWindowId);
    if (activeWindow) {
      if (activeWindow.kind === 'filemanager') {
        focusFileManagerList(state.activeWindowId);
      } else if (activeWindow.kind === 'terminal') {
        const input = desktop.querySelector<HTMLInputElement>(`input[data-terminal-input-window-id="${state.activeWindowId}"]`);
        if (input) {
          input.focus({ preventScroll: true });
        }
      }
    }
  }

  // Restore scroll position to prevent unwanted scrolling
  window.scrollTo(scrollX, scrollY);
};

const importDroppedFiles = async (files: FileList): Promise<void> => {
  // Calculate total size of existing files
  let existingFilesSize = 0;
  for (const entry of fileSystem.values()) {
    existingFilesSize += entry.content.byteLength;
  }

  // Calculate total size of files to be imported
  let importFilesSize = 0;
  const fileContents: { file: File; content: Uint8Array }[] = [];

  for (const file of files) {
    const content = new Uint8Array(await file.arrayBuffer());
    importFilesSize += content.byteLength;
    fileContents.push({ file, content });
  }

  // Check size limit (1.5 MiB)
  const totalSize = existingFilesSize + importFilesSize;
  if (totalSize > FILE_IMPORT_SIZE_LIMIT_BYTES) {
    const existingMiB = (existingFilesSize / (1024 * 1024)).toFixed(2);
    const importMiB = (importFilesSize / (1024 * 1024)).toFixed(2);
    const limitMiB = (FILE_IMPORT_SIZE_LIMIT_BYTES / (1024 * 1024)).toFixed(1);
    const errorMessage = `ファイルの取り込みに失敗しました: 合計サイズ (${existingMiB} MiB + ${importMiB} MiB) が ${limitMiB} MiB の制限を超えています`;
    showErrorDialog(errorMessage);
    return;
  }

  const results: string[] = [];

  for (const { file, content } of fileContents) {
    const sourceName = normalizePathLikeName(file.webkitRelativePath || file.name);
    const result = upsertFile(sourceName, content);
    if (!result.ok) {
      results.push(result.reason);
      continue;
    }

    results.push(`Imported ${result.name} (${formatBytes(content.byteLength)})\n`);
  }

  for (const message of results) {
    emitGlobalTerminalLine(message);
  }

  // ファイルマネージャが開いている場合は再レンダリング
  if (fileManagerWindowId !== null) {
    renderWindows();
  }
};

desktop.addEventListener('dragover', (event) => {
  event.preventDefault();
});

desktop.addEventListener('drop', (event) => {
  event.preventDefault();
  const files = event.dataTransfer?.files;
  if (!files || files.length === 0) {
    return;
  }

  void importDroppedFiles(files);
});

const focusFileManagerList = (windowId: WindowId): void => {
  const list = desktop.querySelector<HTMLDivElement>(`[data-file-manager-list-window-id="${windowId}"]`);
  if (list) {
    // Use setTimeout to ensure focus is set after render completes
    setTimeout(() => {
      list.focus({ preventScroll: true });
    }, 0);
  }
};

const createFileManagerWindow = (): void => {
  // If file manager window already exists, bring it to front
  if (fileManagerWindowId && findWindowById(fileManagerWindowId)) {
    bringToFrontIfNeeded(fileManagerWindowId);
    focusFileManagerList(fileManagerWindowId);
    return;
  }

  const id = createWindowId();

  // Calculate position: use last position if available, otherwise right-top
  let position: { x: number; y: number };
  if (lastFileManagerPosition) {
    position = lastFileManagerPosition;
  } else {
    // Right-top position (20px margin from right edge)
    const rect = desktop.getBoundingClientRect();
    const desktopWidth = Math.max(0, Math.floor(rect.width));
    const rightMargin = 20;
    position = {
      x: Math.max(0, desktopWidth - 300 - rightMargin),
      y: INITIAL_TERMINAL_Y,
    };
  }

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.FILE_MANAGER,
    kind: 'filemanager',
    title: 'File System',
    x: position.x,
    y: position.y,
    width: 300,
    height: 400,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  fileManagerWindowId = id;
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
  focusFileManagerList(id);
};

hamburgerMenu.innerHTML = iconMenu;
hamburgerMenu.addEventListener('click', toggleMenu);

menuNewTerminal.addEventListener('click', () => {
  createTerminalWindow();
  closeMenu();
});

menuFile.addEventListener('click', () => {
  createFileManagerWindow();
  closeMenu();
});

/**
 * [Deprecated] テキストビューアウィンドウの作成関数
 * 
 * 現在は未使用（.txt ファイルは tview コマンド経由で実行される）
 * 将来的にテキストビューア UI を復活させる場合に使用予定
 */
// @ts-expect-error TS6133: Function is deprecated, kept for future use
const createTextViewerWindow = (filename: string): void => {
  // If text viewer for this file already exists, bring it to front
  const existingWindowId = textViewerWindowByFilename.get(filename);
  if (existingWindowId && findWindowById(existingWindowId)) {
    bringToFrontIfNeeded(existingWindowId);
    return;
  }

  const id = createWindowId();
  const centered = getCenteredWindowPosition(600, 400);
  const offset = textViewerWindowByFilename.size * TERMINAL_WINDOW_OFFSET_STEP;

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.TEXT_VIEWER,
    kind: 'textviewer',
    title: filename,
    x: Math.min(centered.x + offset, window.innerWidth - 200),
    y: Math.min(centered.y + offset, window.innerHeight - TASKBAR_HEIGHT_PX - 100),
    width: 600,
    height: 400,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  textViewerWindowByFilename.set(filename, id);
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
};

const createAboutWindow = (): void => {
  // If about window already exists, bring it to front
  if (aboutWindowId && findWindowById(aboutWindowId)) {
    bringToFrontIfNeeded(aboutWindowId);
    return;
  }

  const id = createWindowId();
  const centered = getCenteredWindowPosition(450, 350);

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.ABOUT,
    kind: 'about',
    title: 'About',
    x: centered.x,
    y: centered.y,
    width: 450,
    height: 350,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  aboutWindowId = id;
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
};

const createOnboardingWindow = (): void => {
  // If onboarding window already exists, bring it to front
  if (onboardingWindowId && findWindowById(onboardingWindowId)) {
    bringToFrontIfNeeded(onboardingWindowId);
    return;
  }

  const id = createWindowId();
  const centered = getCenteredWindowPosition(640, 546);

  const windowModel: WindowModel = {
    id,
    appId: APP_IDS.ONBOARDING,
    kind: 'onboarding',
    title: 'ようこそ',
    x: centered.x,
    y: centered.y,
    width: 640,
    height: 546,
    zIndex: state.nextZIndex,
    isActive: true,
  };

  onboardingWindowId = id;
  state.nextZIndex += 1;
  state.activeWindowId = id;
  createWindowModel(windowModel);
  renderWindows();
};

menuAbout.addEventListener('click', () => {
  createAboutWindow();
  closeMenu();
});

desktop.addEventListener('click', (event) => {
  if (state.isMenuOpen && event.target !== hamburgerMenu && !menuPopup.contains(event.target as Node)) {
    closeMenu();
  }

  // ドロップダウンを閉じる
  if (!(event.target as Element).closest('.taskbar-button-group')) {
    state.windowGroups.forEach(g => g.isExpanded = false);
  }
});

// Keyboard input forwarding to Canvas window Worker
document.addEventListener('keydown', (event: KeyboardEvent) => {
  // Update modifier bitmap
  const bitmap = updateModifierBitmap(event, true);

  // Forward to Canvas window Worker if Canvas is active
  if (state.activeWindowId) {
    const activeWindow = findWindowById(state.activeWindowId);
    if (activeWindow && activeWindow.kind === 'canvas') {
      const worker = workerByWindowId.get(state.activeWindowId);
      if (worker) {
        const keyboardEvent: KeyboardEventMessage = {
          type: 'keyboardEvent',
          windowId: state.activeWindowId,
          eventType: 'keydown',
          key: event.key,
          code: event.code,
          keyCode: event.keyCode,
          ctrlKey: event.ctrlKey,
          shiftKey: event.shiftKey,
          altKey: event.altKey,
          metaKey: event.metaKey,
          isAutoRepeat: event.repeat,
          modifierBitmap: bitmap,
        };
        worker.postMessage(keyboardEvent);
        event.preventDefault();
        return;
      }
    }
  }

  // Handle menu Escape key
  if (event.key === 'Escape' && state.isMenuOpen) {
    closeMenu();
  }
});

document.addEventListener('keyup', (event: KeyboardEvent) => {
  // Update modifier bitmap
  const bitmap = updateModifierBitmap(event, false);

  // Forward to Canvas window Worker if Canvas is active
  if (state.activeWindowId) {
    const activeWindow = findWindowById(state.activeWindowId);
    if (activeWindow && activeWindow.kind === 'canvas') {
      const worker = workerByWindowId.get(state.activeWindowId);
      if (worker) {
        const keyboardEvent: KeyboardEventMessage = {
          type: 'keyboardEvent',
          windowId: state.activeWindowId,
          eventType: 'keyup',
          key: event.key,
          code: event.code,
          keyCode: event.keyCode,
          ctrlKey: event.ctrlKey,
          shiftKey: event.shiftKey,
          altKey: event.altKey,
          metaKey: event.metaKey,
          isAutoRepeat: event.repeat,
          modifierBitmap: bitmap,
        };
        worker.postMessage(keyboardEvent);
        event.preventDefault();
        return;
      }
    }
  }
});

loadFileSystem();
createTerminalWindow();
createFileManagerWindow();
createOnboardingWindow();
