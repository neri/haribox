import iconAppWindow from '../assets/icons/app-window.svg?raw';
import { APP_IDS, TITLE_BAR_HEIGHT } from '../constants';
import type { WorkerCommand } from '../protocol';
import { desktop } from '../shell/dom';
import { registerWindowKind } from '../wm/registry';
import { renderWindows } from '../wm/render';
import { state } from '../wm/state';
import type { WindowId, WindowModel } from '../wm/state';
import { bringToFrontIfNeeded, createWindowModel, findWindowById, getCenteredWindowPosition } from '../wm/windowManager';

type CanvasImage = {
  x: number;
  y: number;
  width: number;
  height: number;
  pixels: ArrayBuffer;
};

// Full-window pixel buffer per canvas window, so the canvas can be restored after a re-render
const canvasImageByWindow = new Map<WindowId, CanvasImage>();

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

/** Merges a partial image from a task into the window's buffer and paints it */
export const applyDrawImage = (command: Extract<WorkerCommand, { type: 'drawImage' }>): void => {
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

/** Opens a canvas window for a task. width and height are the size of the canvas, without the title bar */
export const openRustWindow = (windowId: WindowId, width: number, height: number, title: string): void => {
  const existing = findWindowById(windowId);
  if (existing) {
    bringToFrontIfNeeded(windowId);
    return;
  }

  const actualWidth = Math.max(TITLE_BAR_HEIGHT * 4, width);
  const actualHeight = Math.max(TITLE_BAR_HEIGHT + 20, height + TITLE_BAR_HEIGHT);
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

export const registerCanvasKind = (): void => {
  registerWindowKind('canvas', {
    icon: iconAppWindow,
    typeLabel: 'Task',
    renderContent: createCanvas,
    onClosing: (win) => {
      canvasImageByWindow.delete(win.id);
    },
  });
};
